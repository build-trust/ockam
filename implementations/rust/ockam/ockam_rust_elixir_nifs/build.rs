use std::env::consts;

// When running cargo build from the workspace end, we can't directly
// pass these rustflag only for the nif crate, so I'll be adding the flags
// to this file.
fn main() {
    if consts::OS == "macos" {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
