# ockam

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides all dependencies needed to use Ockam in your application.

Types from other Ockam crates are re-exported by this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam = "0.0.0"
```

## Crate Features

The `ockam` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `no_std` context this feature can
disabled as follows

```
[dependencies]
ockam = { version = "0.0.0", default-features = false }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`ockam` default features, Cargo will build `ockam` with the std
feature enabled whether or not your direct dependency on `ockam`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam.svg
[crate-link]: https://crates.io/crates/ockam

[docs-image]: https://docs.rs/ockam/badge.svg
[docs-link]: https://docs.rs/ockam

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
