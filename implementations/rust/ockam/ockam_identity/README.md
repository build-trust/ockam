# ockam_identity

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate supports the domain of "identities", which is required to create secure channels:

 - the `identity` module describes an entity as a set of verified key changes and an identifier
   uniquely representing those changes

 - the `identities` module provides services to create, update, and import identities

 - the `credential` module describes sets of attributes describing a given identity and signed by
   another identity

 - the `credentials` module provides services to create, import and verify credentials

 - the `secure_channel` module describes the steps required to establish a secure channel
   between 2 identities

 - the `secure_channels` module provides services to create a secure channel between 2 identities

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_identity = "0.126.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_identity.svg
[crate-link]: https://crates.io/crates/ockam_identity

[docs-image]: https://docs.rs/ockam_identity/badge.svg
[docs-link]: https://docs.rs/ockam_identity

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
