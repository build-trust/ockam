# ockam_entity

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

Entity is an abstraction over Profiles and Vaults, easing the use of these primitives in authentication and authorization APIs.

## Crate Features

Features of the `ockam_entity` crate:
- `noise_xx` - Enable Noise Protocol XX key agreement dependency.
- `software_vault` - Enable Software Vault dependency.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_entity = "0.34.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam
[ockam-entity-crate-link]: https://crates.io/crates/ockam_entity

[crate-image]: https://img.shields.io/crates/v/ockam_entity.svg
[crate-link]: https://crates.io/crates/ockam_entity

[docs-image]: https://docs.rs/ockam_entity/badge.svg
[docs-link]: https://docs.rs/ockam_entity

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
