# ockam_lease

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides the ability to manage secrets with leases.
A lease is metadata about a secret that indicates a validity period, renewability, and tags.
The use case is wrapping a token to an external service which doesn’t support such features.
Leases will wrap the token such that the token is guaranteed to be valid while the lease is valid.
A lease is revoked at the end of the time duration. Leases may support renewals such as extending the validity period.
Leases can be revoked at any time by the issuing party.

The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_lease = "0.0.0"
```

## Crate Features

The `ockam_lease` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `"no_std"` context you can disable default
features and then enable the `"no_std"` feature as follows:

```
[dependencies]
ockam_lease = { version = "0.1.0", default-features = false, features = ["no_std"] }
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_lease.svg
[crate-link]: https://crates.io/crates/ockam_lease

[docs-image]: https://docs.rs/ockam_lease/badge.svg
[docs-link]: https://docs.rs/ockam_lease

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
