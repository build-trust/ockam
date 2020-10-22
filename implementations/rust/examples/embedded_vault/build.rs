use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Put the linker script somewhere the linker can find it.
fn main() {
    let out_dir = env::var("OUT_DIR").expect("No out dir");
    let dest_path = Path::new(&out_dir);
    let mut f = File::create(&dest_path.join("memory.x"))
        .expect("Could not create file");

    f.write_all(include_bytes!("memory.x"))
        .expect("Could not write file");

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
