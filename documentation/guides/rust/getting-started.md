---
title: Start Here
order: 2
---

# Get Started with the Ockam Rust SDK

This guide walks you through the steps necessary to use the Ockam Rust SDK.

## Development Environment

The Ockam Rust SDK supports the rust `stable` branch.

1. Install rust and cargo using your preferred method, such as [rustup](https://rustup.rs/).
1. Our examples build an "echo service", so `echo` may be a good name for your project. Create a new cargo application project: `cargo new echo`

## Adding The Ockam Dependencies

### Ockam SDK Structure

The Ockam Rust SDK is divided into multiple crates.

- [ockam](https://crates.io/crates/ockam) - The core Ockam API and dependencies.
- [ockam_node](https://crates.io/crates/ockam_node) - an asynchronous worker environment called a Node. The Worker API is central to building
  applications on an Ockam network.
- [ockam_transport_tcp](https://crates.io/crates/ockam_transport_tcp) - TCP implementation of the Ockam Transport protocol.
- [ockam_vault](https://crates.io/crates/ockam_vault) - A software-only Ockam Vault implementation.

The SDK supports the following features:

- `std` (default) - Use the Rust `std` feature.
- `alloc` - Use the `alloc` crate. May be used with `no_std`.
- `no_std` - Turn off Rust `std`. Some SDK functionality may not be available.

### Adding Ockam to a Rust Project

Add the Ockam dependencies to your `Cargo.toml`:

```
[dependencies]
ockam = "0"
ockam_node = "0"
ockam_transport_tcp = "0"
ockam_vault = "0"
```

This will provide the core Rust SDK types and traits, including:

- Credentials, Profiles, and Leases.
- The Ockam Vault for secure storage of secrets.
- The Ockam Node, Worker and Context APIs for asynchronous message processing.

## Downloading Example Source Code

All example source code is available in the [ockam_examples](https://crates.io/crates/ockam_examples) crate. You can also browse the examples in the [Ockam
repository](https://github.com/ockam-network/ockam/tree/develop/implementations/rust/ockam/ockam_examples)
