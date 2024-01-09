use cc;
use std::env;
use std::fs::{self};
use std::path::PathBuf;
use std::fs::File;
use std::io::{Write,Read};
use syn::{ForeignItem, Item, Type};
use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Config {
    asn1c: Asn1cConfig,
}

#[derive(Deserialize, Debug)]
struct Asn1cConfig {
    arguments: Vec<String>
}

/// Enhancements to the generated C code
fn c_fixer(header: &PathBuf) {
    // Rust supports only unsafe access to unions. Here we replace all unions with structs.
    // If you want to use unions, see
    // https://rust-lang.github.io/rust-bindgen/using-unions.html#using-the-bindgenunion-type
    let mut file = File::open(&header).expect("Unable to open file");
    let mut src = String::new();
    file.read_to_string(&mut src).expect("Unable to read file");

    let mut lines = src.lines();
    let mut new_src = String::new();
    while let Some(line) = lines.next() {
        // replace union for struct
        if line.contains("union ") {
            let new_line = line.replace("union ", "struct ");
            new_src.push_str(&new_line);
            new_src.push_str("\n");
            continue;
        }

        new_src.push_str(line);
        new_src.push_str("\n");
    }
    // Write the new source back to the file
    let mut file = File::create(&header).expect("Unable to open file");
    file.write_all(new_src.as_bytes())
        .expect("Unable to write file");

    // This is a hack to get around the fact that the asn1c library
    // does not allocate memory in a way that Rust can free it.
    if header.file_name().unwrap() == "constr_TYPE.h" {

        let mut file = File::open(&header).expect("Unable to open file");
        let mut src = String::new();
        file.read_to_string(&mut src).expect("Unable to read file");

        let mut lines = src.lines();
        let mut new_src = String::new();
        let mut found = false;
        while let Some(line) = lines.next() {
            new_src.push_str(line);
            new_src.push_str("\n");
            if !found && line.contains("ber_tlv_len_t left;") {
                found = true;
                new_src.push_str("    int alloc_method;\n");
            }
        }
        // Write the new source back to the file
        let mut file = File::create(&header).expect("Unable to open file");
        file.write_all(new_src.as_bytes())
            .expect("Unable to write file");
        }
}


fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Read config.toml into a Config struct
    let mut config_file = File::open("config.toml").expect("Unable to open config.toml");
    let mut config_string = String::new();
    config_file.read_to_string(&mut config_string).expect("Unable to read config.toml");
    let config: Config = toml::from_str(&config_string).expect("Unable to parse config.toml");

    Command::new("asn1c")
        .args(&[
            config.asn1c.arguments.join(" ").as_str(),
            "-D",
            out_dir.to_str().unwrap(),
            "-no-gen-example",
        ])
        .status()
        .unwrap();

    // Find all the generated C files, eventually modify them
    let mut sources = Vec::new();
    let mut headers = Vec::new();
    if out_dir.is_dir() {
        for entry in fs::read_dir(&out_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    match extension.to_str().unwrap() {
                        "c" => sources.push(PathBuf::from(path)),
                        "h" => {
                            let pb = PathBuf::from(path);
                            c_fixer(&pb);
                            headers.push(pb)
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    let mut cc_builder = cc::Build::new();
    cc_builder.include(&out_dir.to_str().unwrap());

    for source in sources {
        cc_builder.file(&source);
    }

    cc_builder
        .flag("-Wno-unused-parameter")
        .flag("-Wno-missing-field-initializers");
    cc_builder.compile("libasn1codec");

    // Generate Bindings
    let mut builder = bindgen::Builder::default()
        .clang_arg(format!("-I{}", &out_dir.to_str().unwrap()))
        .derive_copy(false)
        .derive_debug(true)
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .derive_default(true);

    for header in headers {
        builder = builder.header(String::from(header.to_str().unwrap()));
    }

    let bindings = builder.generate().expect("Unable to generate bindings");
    let mut bindings_str = bindings.to_string();

    //  Enhance the bindings
    let mut syntax: syn::File = syn::parse_file(&bindings_str).unwrap();
    bindings_fixer(&mut syntax);
    bindings_str = prettyplease::unparse(&syntax);

    // Find typenames (ASN.1 defined types) and add trait implementations
    let types = find_typenames(&syntax);
    add_trait_impls(&mut bindings_str, &types);

    let mut f = File::create(out_dir.join("bindings.rs")).
        expect("Unable to create file bindings.rs");
    write!(f, "{}", bindings_str).expect("Unable to write bindings");
}


fn bindings_fixer(syntax: &mut syn::File) {
    // Remove all the _PR prefixes from the enum constants (CHOICE.present)
    for i in &mut syntax.items {
        if let syn::Item::Mod(mi) = i {
            let mname = mi.ident.to_string();
            if mname.contains("_PR") {
                if let Some((_, mis)) = &mut mi.content {
                    for mi in mis {
                        if let syn::Item::Const(ci) = mi {
                            let cname = ci.ident.to_string();
                            let prefix = mname.to_string()+"_";
                            let newname = cname.replace(prefix.as_str(), "");
                            ci.ident = syn::Ident::new(&newname, ci.ident.span());
                        }
                    }
                }
            }
        }
    }
}

fn find_typenames(syntax: &syn::File) -> Vec<String> {
    let mut ids = Vec::<String>::new();

    let n = "asn_DEF_".len();

    for i in &syntax.items {
        if let Item::ForeignMod(ifm) = i {
            for fi in &ifm.items {
                if let ForeignItem::Static(fis) = fi {
                    if let Type::Path(path) = &*fis.ty {
                        if let Some(ident) = path.path.get_ident() {
                            if ident == "asn_TYPE_descriptor_t" {
                                let typename = fis.ident.to_string()[n..].to_string();
                                if is_struct(&typename, &syntax) {
                                    ids.push(typename);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ids
}

fn is_struct(typename: &str, syntax: &syn::File) -> bool {
    syntax
        .items
        .iter()
        .any(|item| 
            if let Item::Struct(is) = item {
                is.ident.to_string() == typename
            } else {
                false
            }
        )
}

fn add_trait_impls(bindings: &mut String, typenames: &Vec<String>) {
    bindings.push_str("\n\nuse std::ffi::{CStr, c_void};\n\n");
    for typename in typenames {
        let ti = trait_impl(typename);
        bindings.push_str(&ti);
    }
}

fn trait_impl(typename: &str) -> String {

    let template = r###"

impl Drop for {TYPENAME} {
    fn drop(&mut self) {
        match self._asn_ctx.alloc_method {
            1 => {
                unsafe {
                    let descriptor = &asn_DEF_{TYPENAME};
                    let ops = descriptor.op.as_ref().unwrap();
                    let free_fn = ops.free_struct.unwrap();
                    free_fn(
                        descriptor,
                        self as *mut _ as *mut c_void,
                        asn_struct_free_method::ASFM_FREE_UNDERLYING,
                    );
                }
            }
            0 => {},
            _ => panic!("Invalid alloc_method"),
        }
    }
}

impl Clone for {TYPENAME} {
    fn clone(&self) -> Self {
        let mut tmpv = vec![0u8; 2048]; // TODO: make this dynamic?
        let mut new = Self::default();
        self.encode(EncodingRules::ber, &mut tmpv).unwrap();
        new.decode(EncodingRules::ber, &tmpv).unwrap();
        new
    }
}

impl Coder for {TYPENAME} {
    fn encode(&self, encoding: EncodingRules, buffer: &mut [u8]) -> Result<usize, &'static str> {
    // Get self as a void pointer
    let message_ptr: *const c_void = self as *const _ as *const c_void;
    let encode_buffer_ptr: *mut c_void = buffer.as_mut_ptr() as *mut _ as *mut c_void;
    unsafe {
        let enc_rval = asn_encode_to_buffer(
            std::ptr::null(),
            encoding as u32,
            &asn_DEF_{TYPENAME},
            message_ptr,
            encode_buffer_ptr,
            buffer.len() as usize,
        );
        if enc_rval.encoded != -1 {
            let num_bytes = (enc_rval.encoded) as usize;
            Ok(num_bytes)
        } else {
            Err(CStr::from_ptr((*enc_rval.failed_type).name).to_str().unwrap())
        }
    }
    }

    fn decode(&mut self, encoding: EncodingRules, buffer: &[u8]) -> Result<usize, DecoderError> {
        let mut voidp: *mut c_void = self as *const _ as *mut c_void;
        let voidpp: *mut *mut c_void = &mut voidp;
        self._asn_ctx.alloc_method = 1;
        unsafe {
            let rval = asn_decode(
                std::ptr::null(),
                encoding as u32,
                &asn_DEF_{TYPENAME},
                voidpp,
                buffer.as_ptr() as *const ::std::os::raw::c_void,
                buffer.len() as usize,
            );
            if rval.code == asn_dec_rval_code_e::RC_OK {
                Ok(rval.consumed as usize)
            } else {
                /* Returns Err with FFI integer error code as a DecoderError enum */
                Err(DecoderError::from_u32(rval.code))
            }
        }
    }
}
"###;

    template.replace("{TYPENAME}", typename)
}

