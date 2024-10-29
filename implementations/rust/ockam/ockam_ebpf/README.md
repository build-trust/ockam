# ockam_ebpf

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate contains the eBPF part of Ockam Reliable TCP Portals.

### Build

```bash
cargo build-ebpf
```

Building eBPFs have roughly following requirements:
 - Linux
 - Rust nightly
 - Some dependencies to be installed

Because of that crate with the eBPF code is kept out of the workspace.
Example of a virtual machine to build it can be found in `ubuntu_x86.yaml`.

Using ockam with eBPFs requires:
 - Linux
 - root (CAP_BPF, CAP_NET_RAW, CAP_NET_ADMIN, CAP_SYS_ADMIN)

Example of a virtual machine to run ockam with eBPF can be found in `ubuntu_arm.yaml`.

eBPF is a small architecture-independent object file that is small enough,
to include it in the repo.

The built eBPF object should be copied to `/implementations/rust/ockam/ockam_ebpf/ockam_ebpf`,
from where it will be grabbed by `ockam_transport_tcp` crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_ebpf = "0.1.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_ebpf.svg
[crate-link]: https://crates.io/crates/ockam_ebpf

[docs-image]: https://docs.rs/ockam_ebpf/badge.svg
[docs-link]: https://docs.rs/ockam_ebpf

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
