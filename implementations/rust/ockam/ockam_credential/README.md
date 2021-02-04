# ockam_credentials

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam Credentials is a library for describing, creating, and verifying credentials
on Ockam networks.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_credentials = "0.0.0"
```

## Crate Features

The `ockam_credentials` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `no-std` context this feature can
disabled as follows

```
[dependencies]
ockam_credentials = { version = "0.0.0", default-features = false, features = ["no-std"]  }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`ockam_credentials` default features, Cargo will build `ockam_credentials` with the std
feature enabled whether or not your direct dependency on `ockam_credentials`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_credentials.svg
[crate-link]: https://crates.io/crates/ockam_credentials

[docs-image]: https://docs.rs/ockam_credentials/badge.svg
[docs-link]: https://docs.rs/ockam_credentials

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
