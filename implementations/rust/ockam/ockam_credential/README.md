# ockam_credential

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides the ability to issue and verify attribute based,
privacy preserving, anonymous credentials.

The issuer of a credential signs a collection of statements that attest to
attributes of the subject of that credential. The subject (or a holder on
their behalf) can then selectively disclose these signed statements to a
verifier by presenting a cryptographic proof of knowledge of the issuer's
signature without revealing the actual signature or any of the other
statements that they didn't wish to disclose to this verifier.

Applications can decide if a subject is authorized to take an action based
on the attributes of the subject that were proven to be signed by trusted
issuers. Since only limited and necessary information is revealed about
subjects this improves efficiency, security and privacy of applications.

The main [Ockam][main-ockam-crate-link] crate re-exports types defined in
this crate.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_credential = "0.1.0"
```

## Crate Features

The `ockam_credential` crate has a Cargo feature named `"std"` that is enabled by
default. In order to use this crate in a `"no_std"` context you can disable default
features and then enable the `"no_std"` feature as follows:

```
[dependencies]
ockam_credential = { version = "0.1.0", default-features = false, features = ["no_std"] }
```

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
