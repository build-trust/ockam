# ockam_vault_sync_core

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate contains syncronous wrapper that allows to have multiple Vault instances,
that talk to the same Vault implementation without need for synchronization primitives.

The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_vault_sync_core = "0.38.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_vault_sync_core.svg
[crate-link]: https://crates.io/crates/ockam_vault_sync_core

[docs-image]: https://docs.rs/ockam_vault_sync_core/badge.svg
[docs-link]: https://docs.rs/ockam_vault_sync_core

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
