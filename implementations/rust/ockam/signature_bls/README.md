# signature_bls

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

In order to support a variety of proving protocols, this crate implements the BLS signatures scheme which can be used as a building block for other more elaborate zero-knowledge capable signatures like short group signatures.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
signature_bls = "0.34.0"
```

## Crate Features

The `signature_bls` crate has a Cargo feature named `"alloc"` that is enabled by
default. In order to use this crate in a `no_std` context this feature can
disabled as follows

```
[dependencies]
signature_bls = { version = "0.34.0" , default-features = false }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`signature_bls` default features, Cargo will build `signature_bls` with the std
feature enabled whether or not your direct dependency on `signature_bls`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam
[ockam-vault-crate-link]: https://crates.io/crates/signature_bls

[crate-image]: https://img.shields.io/crates/v/signature_bls.svg
[crate-link]: https://crates.io/crates/signature_bls

[docs-image]: https://docs.rs/signature_bls/badge.svg
[docs-link]: https://docs.rs/signature_bls

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
