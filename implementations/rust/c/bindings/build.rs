extern crate cmake;

use std::env;
use std::path::PathBuf;

fn cargo_rerun_recursive(dir: &PathBuf) {
    println!("cargo:rerun-if-changed={}", dir.to_str().unwrap());
    for dir in std::fs::read_dir(dir).unwrap() {
        let dir = dir.unwrap();
        let path = dir.path();
        if path.is_dir() {
            cargo_rerun_recursive(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }
}

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

    let source_dir = c_dir.join("ockam");
    cargo_rerun_recursive(&source_dir);

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
        ockam_vault_output.join("build/ockam/log").display()
    );
    println!("cargo:rustc-link-lib=static=ockam_log");

    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output.join("build/ockam/vault").display()
    );
    println!("cargo:rustc-link-lib=static=ockam_vault");

    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output.join("build/ockam/memory").display()
    );
    println!("cargo:rustc-link-lib=static=ockam_memory");

    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output.join("build/ockam/mutex").display()
    );
    println!("cargo:rustc-link-lib=static=ockam_mutex");

    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_output
            .join("build/_deps/cryptoauth-build/lib")
            .display()
    );
    println!("cargo:rustc-link-lib=static=cryptoauth");
}
