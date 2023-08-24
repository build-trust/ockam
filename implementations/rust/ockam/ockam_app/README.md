# ockam_app

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate contains the implementation of the Ockam desktop application.

In order to run the application in development you need to execute:
```sh
# to build the `ockam` executable in the target/debug directory
cargo build

# to build the `ockam_desktop` executable in the target/debug directory and start it
# the overridden tauri configuration renames the package.productName value from "Ockam" to
# "OckamDesktop" so that we don't get any conflict with the command line executable name.
# However when the application is published we keep "Ockam" as a name since this will be the
# MacOS bundle name
cd implementations/rust/ockam/ockam_app; cargo tauri dev -c tauri.conf.dev.json; cd -

```

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_app = "0.1.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_app.svg
[crate-link]: https://crates.io/crates/ockam_app

[docs-image]: https://docs.rs/ockam_app/badge.svg
[docs-link]: https://docs.rs/ockam_app

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
