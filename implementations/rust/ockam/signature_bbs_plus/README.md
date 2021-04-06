# signature_bbs_plus

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

In order to support a variety of proving protocols, this crate implements the BBS+ signature scheme which can be used to generate zero-knowledge proofs about signed attributes and the signatures themselves.

The main [Ockam][main-ockam-crate-link] has optional dependency on this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
signature_bbs_plus = "0.1.1"
```

## Crate Features

```
[dependencies]
signature_bbs_plus = { version = "0.1.1", default-features = false }
```

Please note that Cargo features are unioned across the entire dependency
graph of a project. If any other crate you depend on has not opted out of
`signature_bbs_plus` default features, Cargo will build `signature_bbs_plus` with the std
feature enabled whether or not your direct dependency on `signature_bbs_plus`
has `default-features = false`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam
[ockam-vault-crate-link]: https://crates.io/crates/ockam_signature_bbs

[crate-image]: https://img.shields.io/crates/v/ockam_signature_bbs.svg
[crate-link]: https://crates.io/crates/ockam_signature_bbs

[docs-image]: https://docs.rs/ockam_signature_bbs/badge.svg
[docs-link]: https://docs.rs/ockam_signature_bbs

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
