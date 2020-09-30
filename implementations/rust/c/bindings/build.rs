extern crate cmake;

use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let root = match env::var("OCKAM_ROOT") {
        // Assume we're building in Ockam repo
        Err(_) => {
            let cwd = env::current_dir().unwrap();
            cwd.ancestors()
                .nth(4)
                .expect("unable to find Ockam root directory, try setting OCKAM_ROOT")
                .to_path_buf()
        }
        // Use the root we're given
        Ok(root_dir) => PathBuf::from(root_dir),
    };

    // Construct path to C implementation
    let c_dir = root.join("implementations/c");

    // Build
    let ockam_vault_output = out_dir.join("ockam_vault_atecc608a");
    std::fs::create_dir_all(&ockam_vault_output).unwrap();
    let ockam_vault_output = cmake::Config::new(&c_dir)
        .always_configure(true)
        .build_target("ockam_vault_atecc608a")
        .out_dir(ockam_vault_output)
        .build();

    // Link against built library
    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output
            .join("build/ockam/vault/atecc608a")
            .display()
    );
    println!("cargo:rustc-link-lib=static=ockam_vault_atecc608a");

    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output.join("build/ockam/memory").display()
    );
    println!("cargo:rustc-link-lib=static=ockam_memory");
}
