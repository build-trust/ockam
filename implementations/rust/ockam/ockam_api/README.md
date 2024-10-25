# ockam_api

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate supports the creation of a fully-featured Ockam Node
(see [`NodeManager`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs#L87) in [`src/nodes/service.rs`](https://github.com/build-trust/ockam/blob/2fc6d7714a4e54f8734c172ad6480fedc6e3629c/implementations/rust/ockam/ockam_api/src/nodes/service.rs)).

## Configuration

A `NodeManager` maintains its database and log files on disk in
the `OCKAM_HOME` directory (`~/.ockam`) by default:
```shell
root
├─ database.sqlite
├─ nodes
│  ├─ node1
│  │  ├─ stderr.log
│  │  ├─ stdout.log
│  ├─ node2
│  └─ ...
```

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_api = "0.83.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_api.svg
[crate-link]: https://crates.io/crates/ockam_api

[docs-image]: https://docs.rs/ockam_api/badge.svg
[docs-link]: https://docs.rs/ockam_api

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
