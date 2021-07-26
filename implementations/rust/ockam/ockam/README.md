# ockam

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

End-to-end encrypted communication between edge devices and cloud services.

Get started with our step-by-step [hands-on guide][guide].

## Features

* End-to-end encrypted secure channels.
* Multi-hop, multi-transport, application layer routing.
* Node - an asynchronous worker runtime.
* Workers - actors that can handle routed messages.
* Entities and Profiles.
* Attribute-based credentials with selective disclosure.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam = "0.20.0"
```

## Crate Features

The `ockam` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `no_std` context this feature can
disabled as follows

Additional features of the `ockam` crate:
- `ockam_transport_tcp` - Enable TCP transport dependency.
- `ockam_vault` - Enable the default Software Vault implementation.
- `noise_xx` - Enable Noise Protocol XX key agreement dependency.
- `software_vault` - Enable Software Vault dependency.

```
[dependencies]
ockam = { version = "0.20.0"          , default-features = false }
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

[guide]: https://github.com/ockam-network/ockam/blob/develop/documentation/guides/rust/README.md#rust-guide
