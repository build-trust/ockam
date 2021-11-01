# ockam_node_test_attribute

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides the `#[node]` and `#[node_test]` attribute proc_macros. The `#[node]` macro transforms
an async input main function into a regular output main function that sets up
an ockam node and executes the body of the input function inside the node. The `#[node_test]` does the same
but for test functions.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_node_test_attribute = "0.1.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_node_test_attribute.svg
[crate-link]: https://crates.io/crates/ockam_node_test_attribute

[docs-image]: https://docs.rs/ockam_node_test_attribute/badge.svg
[docs-link]: https://docs.rs/ockam_node_test_attribute

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
