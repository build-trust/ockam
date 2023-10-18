# ockam_macros

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides shared macros to:

 - clone structs asynchronously
 - create an ockam node and access its `Context`
 - write some node-related tests


## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_macros = "0.32.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_macros.svg
[crate-link]: https://crates.io/crates/ockam_macros

[docs-image]: https://docs.rs/ockam_macros/badge.svg
[docs-link]: https://docs.rs/ockam_macros

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
