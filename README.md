# asn1r

Experimental import using `bindgen` (FFI) of [asn1c](https://github.com/vlm/asn1c), one of the most feature-rich open-source ASN.1 compilers, into Rust. 
Should in principle support any ASN.1 feature that `asn1c` supports.

It compiles ASN.1 definitions into Rust structs, also accompanied with encoding/decoding methods.

## Usage

### Building
`asn1c` is required to be installed, namely the more up-to-date @mouse07410's [fork](https://github.com/mouse07410/asn1c) which provides various bugfixes and features.

Configuration of the compiler is currently done through `config.toml`. 
Currently `asn1c` parameters, such as the ASN.1 definition files to be compiled can be defined in this file.

To build the library simply run `cargo build`.
Some tests can be performed with `cargo test`.

### Library
While the original C functions are available, this project also provides a more friendly interface.

Consider the provided `Dog` definition provided in `asn1/example.asn1`.

Encoding (JSON here) works as such,
```rust
// Declare a buffer to store our encoded output
let mut data = vec![0u8; 1024];

// Declare a Default Dog
let mut dog = Dog::default();
// Fill the Dog
dog.name.fill("Fido");
dog.canSwim = 0;
dog.age = 9;
dog.breed = Breed::Breed_labrador as i64;
dog.favouriteFood.present = Food_PR::wet;
dog.favouriteFood.choice.wet.brand.fill("Yummy");
dog.favouriteFood.choice.wet.moisturePercentage = 80;
dog.favouriteFood.choice.wet.priceKg = 12;
// Encode the Dog
let renc = dog.encode(EncodingRules::jer, &mut data).unwrap();
println!("Encoded {} bytes:\n{}", renc, show(&data));
```
with an output:
```json
Encoded 233 bytes:
{
    "name": "Fido",
    "age": 9,
    "breed": "labrador",
    "favouriteFood": {
        "wet": {
            "brand": "Yummy",
            "moisturePercentage": 80,
            "priceKg": 12
        }
    },
    "canSwim": true
}
```

Decoding is also straightforward,
```rust
// Declare the Dog to be filled
let mut clone = Dog::default();
// Decode
let rdec = clone.decode(EncodingRules::jer, &data).unwrap();
println!("Decoded OK! Consumed {} bytes", rdec);
```
with an output, `Decoded OK! Consumed 233 bytes`.

Current implemented tests expect the above definition to be compiled.

## Acknowledgements
Special thanks to @sjames for his exploratory [work](https://sjames.github.io/articles/2020-04-26-rust-ffi-asn1-codec/).
If you are into Rust and automotive-related software check him out.
