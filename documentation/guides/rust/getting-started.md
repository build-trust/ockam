---
title: Start Here
order: 2
---

# Get Started with the Ockam Rust SDK

This guide walks you through the steps necessary to use the Ockam Rust SDK.

## Development Environment

The Ockam Rust SDK supports the rust `stable` branch.

1. Install rust and cargo using your preferred method, such as [rustup](https://rustup.rs/).
1. Create a new cargo application project: `cargo new ockam_example`

## Adding The Ockam Dependencies

### Ockam SDK Structure

The Ockam Rust SDK is divided into multiple crates. Most application developers will only need to include the top level
`ockam` crate.

The SDK supports the following features:

- `std` (default) - Use the Rust `std` feature.
- `alloc` - Use the `alloc` crate. May be used with `no_std`.
- `no_std` - Turn off Rust `std`. Some SDK functionality may not be available.

### Adding Ockam to a Rust Project

Add this to your `Cargo.toml`:

```
[dependencies]
ockam = "0"
```

This will provide the core Rust SDK including:

- Credentials, Profiles, and Leases.
- The Ockam Vault for secure storage of secrets.
- The Ockam Node, Worker and Context APIs.

## Downloading Example Source Code

All example source code is available in the [ockam_examples](TODO) crate. You can also browse the examples in the ockam
repository: [https://github.com/ockam-network/ockam/tree/develop/implementations/rust/ockam/ockam_examples](https://github.com/ockam-network/ockam/tree/develop/implementations/rust/ockam/ockam_examples)

## Running Examples With Cargo

Examples in the `examples` directory can be run with cargo:

`cargo run --example credentials`

Some examples may also have standalone binaries with source in `src`.
