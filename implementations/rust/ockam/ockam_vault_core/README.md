# ockam_vault_core

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

In order to support a variety of cryptographically capable hardware we maintain loose coupling between our protocols and how a specific building block is invoked in a specific hardware. This is achieved using an abstract Vault trait.

The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_vault_core = "0.10.0"
```

## Crate Features

The `ockam_vault_core` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `no_std` context this feature can
disabled as follows

```
[dependencies]
ockam_vault_core = { version = "0.10.0"     , default-features = false }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`ockam_vault_core` default features, Cargo will build `ockam_vault_core` with the std
feature enabled whether or not your direct dependency on `ockam_vault_core`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_vault_core.svg
[crate-link]: https://crates.io/crates/ockam_vault_core

[docs-image]: https://docs.rs/ockam_vault_core/badge.svg
[docs-link]: https://docs.rs/ockam_vault_core

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
