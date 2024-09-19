use cfg_aliases::cfg_aliases;
use std::process::Command;

fn hash() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}

fn main() {
    hash();
    cfg_aliases! {
        ebpf_alias: { all(target_os = "linux", feature = "ebpf") }
    }
}
