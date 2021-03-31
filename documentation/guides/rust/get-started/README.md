---
title: Start Here
order: 2
---

#### Have questions? Let us help!

**We are here to help.** See the [Guides And Demos](https://github.com/ockam-network/ockam/discussions/1134) in
GitHub Discussions.

# Get Started with the Ockam Rust SDK

This guide walks you through the steps necessary to use the Ockam Rust SDK.

## Development Environment

The Ockam Rust SDK supports the rust `stable` branch.

1. Install rust and cargo using your preferred method, such as [rustup](https://rustup.rs/).
1. Our examples build an "echo service", so `echo_service` may be a good name for your project. Create a new cargo application project: `cargo new echo_service`

### Adding Ockam to a Rust Project

Add the Ockam dependency to your `Cargo.toml`:

```
[dependencies]
ockam = "0"
```

## Working with multiple binaries

Some of the Ockam examples need two programs. There are several ways you can configure your project to have multiple binaries.
The easiest way, and the way that we will use in this guide is to use the `examples` directory.

For example:

1. Create an `examples` directory.
1. Create `client.rs` and `server.rs` source files in the `examples` directory.
1. The programs can be executed using cargo: `cargo run --example server` and `cargo run --example client`.

Now we are ready to [Start building the Echo Service](../01-workers)
