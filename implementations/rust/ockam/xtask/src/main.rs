//! This xtask is designed for targets that require custom scripts to be built.
//! Currently, its only purpose is to build `ockam_ebpf` eBPF object file.
//!
//! ## Build
//!
//! ```bash
//! cargo build-ebpf
//! ```
//!
//! Building eBPFs have roughly following requirements:
//!  - Linux
//!  - Rust nightly
//!  - Some dependencies to be installed
//!
//! Because of that crate with the eBPF code is kept out of the workspace.
//! Example of a virtual machine to build it can be found in `ubuntu_x86.yaml`.
//!
//! Using ockam with eBPFs requires:
//!  - Linux
//!  - root (CAP_BPF, CAP_NET_RAW, CAP_NET_ADMIN, CAP_SYS_ADMIN)
//!
//! Example of a virtual machine to run ockam with eBPF can be found in `ubuntu_arm.yaml`.
//!
//! eBPF is a small architecture-independent object file that is small enough,
//! to include it in the repo.
//!
//! The built eBPF object should be copied to `/implementations/rust/ockam/ockam_ebpf/ockam_ebpf`,
//! from where it will be grabbed by `ockam_transport_tcp` crate.

use std::{path::PathBuf, process::Command};

use clap::Parser;

#[derive(Debug, Copy, Clone)]
pub enum Architecture {
    BpfEl,
    // eBPF code may need to be updated to behave correctly on big-endian (especially checksum calc)
    // BpfEb,
}

impl std::str::FromStr for Architecture {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "bpfel-unknown-none" => Architecture::BpfEl,
            // "bpfeb-unknown-none" => Architecture::BpfEb,
            _ => return Err("invalid target".to_owned()),
        })
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Architecture::BpfEl => "bpfel-unknown-none",
            // Architecture::BpfEb => "bpfeb-unknown-none",
        })
    }
}

#[derive(Debug, Parser, Clone)]
pub struct Options {
    /// Set the endianness of the BPF target
    #[clap(default_value = "bpfel-unknown-none", long)]
    target: Architecture,
    #[clap(long, short, group = "profile_group")]
    release: bool,
    #[clap(long, group = "profile_group")]
    profile: Option<String>,
    #[clap(long)]
    target_dir: Option<PathBuf>,
}

pub fn build_ebpf(opts: Options, dir: PathBuf) {
    let target = format!("--target={}", opts.target);
    let mut args = vec!["build", target.as_str(), "-Z", "build-std=core"];
    if opts.release {
        args.push("--release")
    }

    if let Some(profile) = &opts.profile {
        args.push("--profile");
        args.push(profile);
    }

    if let Some(target_dir) = &opts.target_dir {
        args.push("--target-dir");
        args.push(target_dir.to_str().unwrap());
    }

    // Command::new creates a child process which inherits all env variables. This means env
    // vars set by the cargo xtask command are also inherited. RUSTUP_TOOLCHAIN is removed
    // so the rust-toolchain.toml file in the -ebpf folder is honored.

    let status = Command::new("cargo")
        .current_dir(dir.clone())
        .env_remove("RUSTUP_TOOLCHAIN")
        .args(&args)
        .status()
        .expect("failed to run build bpf program");

    assert!(status.success(), "failed to build bpf program");
}

fn main() {
    let opts = Options::parse();

    let dir = PathBuf::from("implementations/rust/ockam/ockam_ebpf");

    build_ebpf(opts.clone(), dir.clone());
}
