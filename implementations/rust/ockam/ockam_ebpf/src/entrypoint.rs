//! This crate contains the eBPF part of Ockam Reliable TCP Portals.
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

#![no_std]
#![no_main]

use aya_ebpf::macros::classifier;
use aya_ebpf::programs::TcContext;

mod checksum;
mod checksum_helpers;
mod common;
mod conversion;

use crate::common::Direction;

#[classifier]
pub fn ockam_ingress(ctx: TcContext) -> i32 {
    common::try_handle(&ctx, Direction::Ingress).unwrap_or_else(|ret| ret)
}

#[classifier]
pub fn ockam_egress(ctx: TcContext) -> i32 {
    common::try_handle(&ctx, Direction::Egress).unwrap_or_else(|ret| ret)
}

// TODO: Check if eBPF code can panic at all
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
