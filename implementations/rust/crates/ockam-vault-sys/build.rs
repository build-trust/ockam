extern crate bindgen;
extern crate cmake;
extern crate walkdir;
extern crate which;

use std::env;
use std::path::PathBuf;

use bindgen::EnumVariation;
use walkdir::WalkDir;
use which::which;

const ENV_LLVM_PREFIX: &'static str = "LLVM_PREFIX";

fn main() {
    let target = env::var("TARGET").unwrap();
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
    let include_dir = c_dir.join("include");
    let lib_dir = c_dir.join("lib");

    // Build
    let ockam_vault_output = out_dir.join("ockam_vault");
    std::fs::create_dir_all(&ockam_vault_output).unwrap();
    let ockam_vault_path = cmake::Config::new(&c_dir)
        .always_configure(true)
        .define("OCKAM_TARGET_TRIPLE", &target)
        .out_dir(ockam_vault_output)
        .build_target("ockam_vault_default")
        .build();

    // Construct CMake profile name from Rust profile
    let profile = {
        let mut profile = env::var("PROFILE").unwrap();
        let (first, _rest) = profile.split_at_mut(1);
        first.make_ascii_uppercase();
        profile
    };

    // Link against built library
    println!(
        "cargo:rustc-link-search=native={}",
        ockam_vault_path
            .join(&format!("build/{}/lib", profile))
            .display()
    );
    println!("cargo:rustc-link-lib=static=ockam_vault_default");
    // Expose include path to downstream crates via DEP_OCKAM_VAULT_INCLUDE
    println!("cargo:include={}", include_dir.display());
    // Rerun build if any of the include paths change
    let walker = WalkDir::new(&include_dir).into_iter();
    for entry in walker.filter_entry(|e| e.file_type().is_file()) {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().display());
    }

    // Generate bindings if llvm-config is present
    if let Ok(llvm_config) = which("llvm-config") {
        // Rebuild bindings if we modify the wrapper
        println!("cargo:rerun-if-changed=c_src/vault.h");
        generate_bindings(
            llvm_config,
            include_dir,
            lib_dir,
            out_dir.join("bindings.rs"),
        );
        return;
    }

    // Otherwise, try to find llvm-config and generate bindings if found
    if let Some(llvm_prefix) = env::var_os(ENV_LLVM_PREFIX) {
        let llvm_config = PathBuf::from(llvm_prefix).join("bin/llvm-config");
        if llvm_config.exists() {
            println!("cargo:rerun-if-changed=c_src/vault.h");
            generate_bindings(
                llvm_config,
                include_dir,
                lib_dir,
                out_dir.join("bindings.rs"),
            );
            return;
        }
    }

    println!("cargo:rerun-if-env-changed={}", ENV_LLVM_PREFIX);
    println!(
        "cargo:warning={}",
        "LLVM_PREFIX was not set, and cannot find llvm-config, will not regenerate bindings"
    );
}

fn generate_bindings(
    llvm_config: PathBuf,
    include_dir: PathBuf,
    lib_dir: PathBuf,
    out_path: PathBuf,
) {
    env::set_var("LLVM_CONFIG_PATH", &llvm_config);
    let bindings = bindgen::Builder::default()
        .header("c_src/vault.h")
        .use_core()
        .ctypes_prefix("crate::ctypes")
        .detect_include_paths(true)
        .size_t_is_usize(true)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .prepend_enum_name(false)
        .layout_tests(false)
        .ignore_methods()
        .bitfield_enum("OckamFeatures")
        .whitelist_function("(Ockam|Vault).*")
        .whitelist_type("(kOckam|Ockam|OCKAM|Vault|VAULT).*")
        .whitelist_var("(kOckam|Ockam|OCKAM|Vault|VAULT).*")
        .clang_arg("-I")
        .clang_arg(include_dir.to_str().unwrap())
        .clang_arg("-I")
        .clang_arg(lib_dir.to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings to Ockam Vault!");

    bindings.write_to_file(out_path).unwrap();
}
