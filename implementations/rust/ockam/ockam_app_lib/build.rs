extern crate cbindgen;

use cbindgen::{Config, Error};
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_file = "../../../swift/ockam/ockam_app/Ockam/Bridge.h";
    let config = Config::from_file("cbindgen.toml").unwrap();

    let result = cbindgen::generate_with_config(crate_dir, config);

    match result {
        Ok(bindings) => {
            bindings.write_to_file(output_file);
        }
        Err(error) => {
            match error {
                Error::ParseSyntaxError { .. } | Error::ParseCannotOpenFile { .. } => {
                    //compilation failed, if we panic no meaningful error will be reported
                    eprintln!("Failed to generate C bindings: {}", error);
                }
                _ => {
                    panic!("Failed to generate C bindings: {}", error);
                }
            }
        }
    }
}
