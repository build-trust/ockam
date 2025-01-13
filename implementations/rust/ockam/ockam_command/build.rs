use cfg_aliases::cfg_aliases;
use std::env;
use std::process::Command;

fn hash() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}

fn binary_name() {
    let bin_name = env::var("OCKAM_COMMAND_BIN_NAME").unwrap_or("ockam".to_string());
    println!("cargo:rustc-env=OCKAM_COMMAND_BIN_NAME={bin_name}");
    println!("cargo:rerun-if-env-changed=OCKAM_COMMAND_BIN_NAME");

    let brand_name = env::var("OCKAM_COMMAND_BRAND_NAME").unwrap_or("Ockam".to_string());
    println!("cargo:rustc-env=OCKAM_COMMAND_BRAND_NAME={brand_name}");
    println!("cargo:rerun-if-env-changed=OCKAM_COMMAND_BRAND_NAME");

    let support_email =
        env::var("OCKAM_COMMAND_SUPPORT_EMAIL").unwrap_or(format!("support@{}.com", bin_name));
    println!("cargo:rustc-env=OCKAM_COMMAND_SUPPORT_EMAIL={support_email}");
    println!("cargo:rerun-if-env-changed=OCKAM_COMMAND_SUPPORT_EMAIL");

    let home_dir = env::var("OCKAM_HOME").unwrap_or("".to_string());
    println!("cargo:rustc-env=OCKAM_HOME={home_dir}");
    println!("cargo:rerun-if-env-changed=OCKAM_HOME");

    let orchestrator_identifier =
        env::var("OCKAM_CONTROLLER_IDENTITY_ID").unwrap_or("".to_string());
    println!("cargo:rustc-env=OCKAM_CONTROLLER_IDENTITY_ID={orchestrator_identifier}");
    println!("cargo:rerun-if-env-changed=OCKAM_CONTROLLER_IDENTITY_ID");

    let orchestrator_address = env::var("OCKAM_CONTROLLER_ADDR").unwrap_or("".to_string());
    println!("cargo:rustc-env=OCKAM_CONTROLLER_ADDR={orchestrator_address}");
    println!("cargo:rerun-if-env-changed=OCKAM_CONTROLLER_ADDR");
}

fn main() {
    hash();
    binary_name();
    cfg_aliases! {
        privileged_portals_support: { all(target_os = "linux", feature = "privileged_portals") }
    }
}
