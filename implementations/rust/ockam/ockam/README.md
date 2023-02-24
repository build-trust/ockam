# ockam

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![status][status-image]][status-link]
[![discuss][discuss-image]][discuss-link]

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

## Features

* End-to-end encrypted, mutually authenticated _secure channels_.
* Multi-hop, multi-transport, application layer routing.
* Key establishment, rotation, and revocation - _for fleets, at scale_.
* Lightweight, Concurrent, Stateful Workers that enable _simple APIs_.
* Attribute-based Access Control - credentials with _selective disclosure_.
* Add-ons for a variety of operating environments, transport protocols, and _cryptographic hardware_.

## Get Started

* [__End-to-End Encryption with Rust__][e2ee-rust-guide]:
In this guide, we create two small Rust programs called Alice and Bob. Alice and Bob send each other
messages, over the network, via a cloud service. They mutually authenticate each other and have a cryptographic
guarantee that the integrity, authenticity, and confidentiality of their messages is protected end-to-end.
[👉][e2ee-rust-guide]

* [__Step-by-Step Deep Dive__][step-by-step-rust-guide]:
In this step-by-step guide we write many small rust programs to understand the various building blocks
that make up Ockam. We dive into Node, Workers, Routing, Transport, Secure Channels and more.
[👉][step-by-step-rust-guide]

* [__End-to-End Encryption through Kafka__][e2ee-kafka-guide]:
In this guide, we show two programs called Alice and Bob. Alice and Bob send each other messages, over
the network, via a cloud service, _through Kafka_. They mutually authenticate each other and have a
cryptographic guarantee that the integrity, authenticity, and confidentiality of their messages is protected
end-to-end. The Kafka instance, the intermediary cloud service and attackers on the network are not be able
to see or change the contents of en-route messages. The application data in Kafka is encrypted.
[👉][e2ee-kafka-guide]

* [__Build Secure Remote Access Tunnels__][secure-remote-access-tunnels]:
In this guide, we'll write a few simple Rust programs to programmatically create secure access tunnels to remote
services and devices that are running in a private network, behind a NAT. We'll then tunnel arbitrary communication
protocols through these secure tunnels.
[👉][secure-remote-access-tunnels]

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam = "0.81.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam.svg
[crate-link]: https://crates.io/crates/ockam

[docs-image]: https://docs.rs/ockam/badge.svg
[docs-link]: https://docs.rs/ockam

[status-image]: https://img.shields.io/badge/Status-Preview-58E0C9.svg
[status-link]: https://github.com/build-trust/ockam/blob/develop/SECURITY.md

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-On%20Github-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions

[e2ee-rust-guide]: https://github.com/build-trust/ockam/tree/develop/documentation/use-cases/end-to-end-encryption-with-rust#readme
[e2ee-kafka-guide]: https://github.com/build-trust/ockam/tree/develop/documentation/use-cases/end-to-end-encryption-through-kafka#readme
[step-by-step-rust-guide]: https://github.com/build-trust/ockam/tree/develop/documentation/guides/rust#readme
[secure-remote-access-tunnels]: https://github.com/build-trust/ockam/tree/develop/documentation/use-cases/secure-remote-access-tunnels
