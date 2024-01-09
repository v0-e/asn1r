#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(improper_ctypes)] /* FFI u128 */

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub trait Coder {
    /// Encode the type into the buffer. The buffer must be large enough
    fn encode(&self, encoding: EncodingRules, buffer: &mut [u8]) -> Result<usize, &'static str>;

    /// Decode the type from the buffer.
    fn decode(&mut self, encoding: EncodingRules, buffer: &[u8]) -> Result<usize, DecoderError>;
}

enum AllocMethod {
    /// Allocated in Rust
    application = 0,
    /// Allocated by the decoder
    native = 1,
}

#[derive(Debug)]
pub enum DecoderError {
    /// Decoder wants more data
    want_more = asn_dec_rval_code_e::RC_WMORE as isize,
    /// General error
    failure = asn_dec_rval_code_e::RC_FAIL as isize,
}

impl DecoderError {
    fn from_u32(code: u32) -> Self {
        match code {
            1 => DecoderError::want_more,
            2 => DecoderError::failure,
            _ => panic!("Unknown error code"),
        }
    }
}

#[derive(Debug)]
pub enum EncodingRules {
    /// Basic Encoding Rules
    ber = asn_transfer_syntax::ATS_BER as isize,
    /// Distinguished Encoding Rules
    der = asn_transfer_syntax::ATS_DER as isize,
    /// Canonical Encoding Rules
    cer = asn_transfer_syntax::ATS_CER as isize,
    /// Octet Encoding Rules
    oer = asn_transfer_syntax::ATS_BASIC_OER as isize,
    /// Canonical Octet Encoding Rules
    coer = asn_transfer_syntax::ATS_CANONICAL_OER as isize,
    /// Unaligned Packed Encoding Rules,
    uper = asn_transfer_syntax::ATS_UNALIGNED_BASIC_PER as isize,
    /// Canonical Unaligned Packed Encoding Rules,
    cuper = asn_transfer_syntax::ATS_UNALIGNED_CANONICAL_PER as isize,
    /// Aligned Packed Encoding Rules,
    aper = asn_transfer_syntax::ATS_ALIGNED_BASIC_PER as isize,
    /// Canonical Aligned Packed Encoding Rules,
    caper = asn_transfer_syntax::ATS_ALIGNED_CANONICAL_PER as isize,
    /// XML Encoding Rules
    xer = asn_transfer_syntax::ATS_BASIC_XER as isize,
    /// Canonical XML Encoding Rules
    cxer = asn_transfer_syntax::ATS_CANONICAL_XER as isize,
    /// JSON Encoding Rules
    jer = asn_transfer_syntax::ATS_JER as isize,
    /// JSON Encoding Rules (minified)
    jerm = asn_transfer_syntax::ATS_JER_MINIFIED as isize,
}
    
impl OCTET_STRING {
    pub fn fill(&mut self, s: &str) {
        self.buf = s.as_ptr() as *mut u8;
        self.size = s.len();
    }
}

#[cfg(test)]
mod tests {
 
    use super::*;

    fn show(bs: &[u8]) -> String {
        String::from_utf8_lossy(bs).into_owned()
    }

    #[test]
    fn roundtrip() {
        let mut data = vec![0u8; 1024];

        // Lets use a random Dog for testing
        let mut dog = Dog::default();

        dog.name.fill("Fido");
        dog.canSwim = 0;
        dog.age = 9;
        dog.breed = Breed::labrador as i64;

        dog.favouriteFood.present = Food_PR::wet;
        dog.favouriteFood.choice.wet.brand.fill("Yummy");
        dog.favouriteFood.choice.wet.moisturePercentage = 80;
        dog.favouriteFood.choice.wet.priceKg = 12;
 
        let mut records = Dog_records::default();
        records.list.count = 2;
        records.list.size = 16;

        let mut record0 = Record::default();
        record0.description.fill("Found a bone");
        record0.date.fill("20220905220600.000");
        let mut record1 = Record::default();
        record1.description.fill("Went to the doctor");
        record1.date.fill("20231106210629.456");

        let mut arr: Vec<&Record> = Vec::with_capacity(2);
        arr.push(&record0);
        arr.push(&record1);

        records.list.array = arr.as_mut_ptr() as *mut *mut Record;
        dog.records = &mut records;

        // Encode
        let renc = dog.encode(EncodingRules::jer, &mut data);
        match renc {
            Ok(b) => {
                println!("Encoded {} bytes:\n{}", b, show(&data));
            },
            Err(e) => panic!("Encode failed: {}", e),
        }

        // Decode
        let mut clone = Dog::default();
        let decoded_bytes = clone.decode(EncodingRules::jer, &data);
        match decoded_bytes {
            Ok(b) => {
                println!("Decoded OK! Consumed {} bytes", b);
            },
            Err(e) => panic!("Decode failed: {:?}", e),
        }

        assert_eq!(dog.age, clone.age);
        assert_eq!(dog.breed, clone.breed);
        assert_eq!(dog.favouriteFood.present, clone.favouriteFood.present);
        assert_eq!(dog.favouriteFood.choice.wet.moisturePercentage, 
            clone.favouriteFood.choice.wet.moisturePercentage);
    }

    // Deep-copy test
    #[test]
    fn clone() {
        // Lets use a random Dog for testing
        let mut dog = Dog::default();

        dog.name.fill("Fido");
        dog.canSwim = 0;
        dog.age = 9;
        dog.breed = Breed::labrador as i64;

        dog.favouriteFood.present = Food_PR::wet;
        dog.favouriteFood.choice.wet.brand.fill("Yummy");
        dog.favouriteFood.choice.wet.moisturePercentage = 80;
        dog.favouriteFood.choice.wet.priceKg = 12;
 
        let mut records = Dog_records::default();
        records.list.count = 2;
        records.list.size = 16;

        let mut record0 = Record::default();
        record0.description.fill("Found a bone");
        record0.date.fill("20220905220600.000");
        let mut record1 = Record::default();
        record1.description.fill("Went to the doctor");
        record1.date.fill("20231106210629.456");

        let mut arr: Vec<&Record> = Vec::with_capacity(2);
        arr.push(&record0);
        arr.push(&record1);

        records.list.array = arr.as_mut_ptr() as *mut *mut Record;
        dog.records = &mut records;

        let clone = dog.clone();

        assert_eq!(dog.age, clone.age);
        assert_eq!(dog.breed, clone.breed);
        assert_eq!(dog.favouriteFood.present, clone.favouriteFood.present);
        assert_eq!(dog.favouriteFood.choice.wet.moisturePercentage, 
            clone.favouriteFood.choice.wet.moisturePercentage);

    }
}
