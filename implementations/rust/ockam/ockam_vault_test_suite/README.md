# ockam_vault_test_suite

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

In order to support a variety of cryptographically capable hardware we maintain loose coupling between our protocols and how a specific building block is invoked in a specific hardware. This is achieved using an abstract Vault trait.

A concrete implementation of the Vault trait is called an Ockam Vault. Over time, and with help from the Ockam open source community, we plan to add vaults for several TEEs, TPMs, HSMs, and Secure Enclaves.

This crate provides a test suite for Ockam Vault implementations. Combined with the `ockam_vault_test_attribute`, this test
suite is decoupled from specific Vault implementations.

The main [Ockam][main-ockam-crate-link] has optional dependency on this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_vault_test_suite = "0.19.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam
[ockam-vault-crate-link]: https://crates.io/crates/ockam_vault

[crate-image]: https://img.shields.io/crates/v/ockam_vault.svg
[crate-link]: https://crates.io/crates/ockam_vault

[docs-image]: https://docs.rs/ockam_vault/badge.svg
[docs-link]: https://docs.rs/ockam_vault

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
