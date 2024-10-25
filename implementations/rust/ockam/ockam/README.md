# ockam

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

End-to-end encrypted, mutually authenticated, secure communication.

_[A hands-on guide 👉][e2ee-rust-guide]_.

Data, within modern distributed applications, are rarely exchanged over a single point-to-point
transport connection. Application messages routinely flow over complex, multi-hop, multi-protocol
routes — _across data centers, through queues and caches, via gateways and brokers_ — before reaching
their end destination.

Transport layer security protocols are unable to protect application messages because their protection
is constrained by the length and duration of the underlying transport connection.

Ockam makes it simple for our applications to guarantee end-to-end integrity, authenticity,
and confidentiality of data. We no longer have to implicitly depend on the defenses of every machine
or application within the same, usually porous, network boundary. Our application's messages don't have
to be vulnerable at every point, along their journey, where a transport connection terminates.

Instead, our application can have a strikingly smaller vulnerability surface and easily make
_granular authorization decisions about all incoming information and commands._

### Features

* End-to-end encrypted, mutually authenticated _secure channels_.
* Multi-hop, multi-transport, application layer routing.
* Key establishment, rotation, and revocation - _for fleets, at scale_.
* Lightweight, Concurrent, Stateful Workers that enable _simple APIs_.
* Attribute-based Access Control - credentials with _selective disclosure_.
* Add-ons for a variety of operating environments, transport protocols, and _cryptographic hardware_.

### Documentation

Tutorials, examples and reference guides are available at [docs.ockam.io](https://docs.ockam.io).

[e2ee-rust-guide]: https://docs.ockam.io/reference/libraries/rust

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam = "0.140.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam.svg
[crate-link]: https://crates.io/crates/ockam

[docs-image]: https://docs.rs/ockam/badge.svg
[docs-link]: https://docs.rs/ockam

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
