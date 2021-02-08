# ockam_credential

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam Credential is a library for describing, creating, and verifying credentials
on Ockam networks.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_credential = "0.0.0"
```

## Crate Features

The `ockam_credential` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `no-std` context this feature can
disabled as follows

```
[dependencies]
ockam_credential = { version = "0.0.0", default-features = false, features = ["no-std"]  }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`ockam_credential` default features, Cargo will build `ockam_credential` with the std
feature enabled whether or not your direct dependency on `ockam_credential`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_credential.svg
[crate-link]: https://crates.io/crates/ockam_credential

[docs-image]: https://docs.rs/ockam_credential/badge.svg
[docs-link]: https://docs.rs/ockam_credential

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
