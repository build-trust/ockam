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
    let is_developer = env::var("COMPILE_OCKAM_DEVELOPER").unwrap_or("false".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_DEVELOPER={is_developer}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_DEVELOPER");

    let bin_name = env::var("COMPILE_OCKAM_COMMAND_BIN_NAME").unwrap_or("ockam".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_COMMAND_BIN_NAME={bin_name}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_COMMAND_BIN_NAME");

    let brand_name = env::var("COMPILE_OCKAM_COMMAND_BRAND_NAME").unwrap_or("Ockam".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_COMMAND_BRAND_NAME={brand_name}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_COMMAND_BRAND_NAME");

    let support_email = env::var("COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL")
        .unwrap_or(format!("support@{}.com", bin_name));
    println!("cargo:rustc-env=COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL={support_email}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_COMMAND_SUPPORT_EMAIL");

    let home_dir = env::var("COMPILE_OCKAM_HOME").unwrap_or("".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_HOME={home_dir}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_HOME");

    let commands = env::var("COMPILE_OCKAM_COMMANDS").unwrap_or("".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_COMMANDS={commands}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_COMMANDS");

    let orchestrator_identifier =
        env::var("COMPILE_OCKAM_CONTROLLER_IDENTIFIER").unwrap_or("".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_CONTROLLER_IDENTIFIER={orchestrator_identifier}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_CONTROLLER_IDENTIFIER");

    let orchestrator_address =
        env::var("COMPILE_OCKAM_CONTROLLER_ADDRESS").unwrap_or("".to_string());
    println!("cargo:rustc-env=COMPILE_OCKAM_CONTROLLER_ADDRESS={orchestrator_address}");
    println!("cargo:rerun-if-env-changed=COMPILE_OCKAM_CONTROLLER_ADDRESS");
}

fn main() {
    hash();
    binary_name();
    cfg_aliases! {
        privileged_portals_support: { all(target_os = "linux", feature = "privileged_portals") }
    }
}
