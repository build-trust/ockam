use std::{env, error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // put memory layout (linker script) in the linker search path
    fs::copy("memory.x", out_dir.join("memory.x"))?;

    println!("cargo:rustc-link-search={}", out_dir.display());

    Ok(())
}
