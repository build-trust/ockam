extern crate bindgen;
extern crate which;

use std::env;
use std::path::PathBuf;

use bindgen::EnumVariation;
use which::which;

const ENV_LLVM_PREFIX: &str = "LLVM_PREFIX";

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

    let mut include_dirs = Vec::<String>::new();
    let modules = vec!["error", "mutex", "memory", "vault", "vault/atecc608a"];

    let include_root_dir = ockam_vault_output.join("build").join("ockam");

    for module in modules {
        let include_dir = include_root_dir.join(module).join("include");

        include_dirs.push(String::from(include_dir.to_str().unwrap()));
    }

    let cryptoauth_include1 = ockam_vault_output
        .join("build")
        .join("_deps")
        .join("cryptoauthlib-src")
        .join("lib");

    include_dirs.push(String::from(cryptoauth_include1.to_str().unwrap()));

    let cryptoauth_include2 = ockam_vault_output
        .join("build")
        .join("_deps")
        .join("cryptoauthlib-build")
        .join("lib");

    include_dirs.push(String::from(cryptoauth_include2.to_str().unwrap()));

    let llvm_config = {
        if let Ok(config) = which("llvm-config") {
            Some(config)
        } else if let Some(llvm_prefix) = env::var_os(ENV_LLVM_PREFIX) {
            let config = PathBuf::from(llvm_prefix).join("bin/llvm-config");
            if config.exists() {
                Some(config)
            } else {
                None
            }
        } else {
            None
        }
    };

    // Generate bindings if llvm-config is present
    if let Some(llvm_config) = llvm_config {
        let src_file = root.join("implementations/rust/c/bindings/src/bindings.rs");
        generate_bindings(llvm_config, include_dirs, &src_file);
    } else {
        println!("cargo:error=LLVM_PREFIX was not set, and cannot find llvm-config, will not regenerate bindings");
    }
}

fn generate_bindings(llvm_config: PathBuf, include_dirs: Vec<String>, out_path: &PathBuf) {
    env::set_var("LLVM_CONFIG_PATH", &llvm_config);
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .use_core()
        .detect_include_paths(true)
        .size_t_is_usize(true)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .prepend_enum_name(false)
        .layout_tests(false)
        .ignore_methods()
        .whitelist_function("(ockam|Ockam).*")
        .whitelist_type("(ockam|kOckam|Ockam|OCKAM).*")
        .whitelist_var("(ockam|kOckam|Ockam|OCKAM).*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks));

    for include_dir in include_dirs {
        builder = builder.clang_arg(format!("-I/{}", include_dir));
    }

    let bindings = builder
        .generate()
        .expect("Failed to generate bindings to Ockam Vault!");

    bindings.write_to_file(out_path).unwrap();
}
