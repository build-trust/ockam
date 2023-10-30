# ockam_node

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides an implementation of an Ockam [Ockam][main-ockam-crate-link]
Node and is intended for use by crates that provide features and add-ons
to the main [Ockam][main-ockam-crate-link] library.

The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
this crate, when the `"std"` feature is enabled.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_node = "0.96.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_node.svg
[crate-link]: https://crates.io/crates/ockam_node

[docs-image]: https://docs.rs/ockam_node/badge.svg
[docs-link]: https://docs.rs/ockam_node

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
