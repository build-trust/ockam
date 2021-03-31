<div align="center">
<em>
<a href="documentation/guides/rust/README.md#rust-guide">Build your first Ockam
application, in 10 mins, with our step-by-step hands-on guide.</a>
</em>
</div>

<hr>

<p>
<a href="implementations#readme">
<img alt="Ockam" src="./documentation/concepts/assets/banner.png" width="900">
</a>
</p>

Rust and Elixir libraries for end-to-end encrypted, mutually authenticated
communication between edge devices and cloud services.

Machines, within the Internet of Things, operate by exchanging messages, with
cloud services and other connected machines. Secure, private, and trustworthy
connected applications must protect these messages from eavesdropping,
tampering, and forgery.

Ockam is a suite of programming libraries and infrastructure that make it
simple for applications to dynamically create end-to-end encrypted, mutually
authenticated, mutually authorized, lightweight secure channels that protect
messages, end-to-end — from their source to their destination.

## Features

* End-to-end encrypted, mutually authenticated secure channels.
* Key Lifecycle and Credential Exchange protocols.
* Attribute-based Access Control with Credentials that support Selective Disclosure.
* Add-Ons for a variety of transport protocols, hardware cryptography, and
  enterprise integrations.
* Libraries for multiple languages - Rust, Elixir (more on the roadmap)

<hr>

## Concepts

<p>
<a href="documentation/concepts/index.md">
<img alt="Ockam" src="./documentation/concepts/assets/ockam-features.png" width="900">
</a>
</p>

### Secure Channels

To protect en-route messages against eavesdropping, tampering, and forgery …
we usually need a cryptographic secure channel protocol.

Most IoT message transport protocols support some way to establish a secure
channel. However, such secure channel protocols have traditionally been tightly
coupled to their corresponding transport protocols. Their security guarantees
are limited by the length and duration of a single transport layer connection.

This constraint, often leads to application architectures that violate the
foundational security principle of least privilege … exposing applications
to a vulnerability and liability surface that is a lot bigger than it needs
to be.

Ockam secure channels are decoupled from the transport layer and instead
use Ockam [Application Layer Routing](#application-layer-routing) to provide
end-to-end data integrity and confidentiality.

### Application Layer Routing

It is common, for messages in intelligent, connected applications, to traverse
a complex path that isn’t a simple point-to-point transport protocol connection.

To support occasionally connected devices, low power radio protocols and
containerized microservices … messages usually travel via a number of message
queues and caches, often over a series of network layer connections … before
reaching their end destination.

Ockam Application Layer Routing is a compact binary protocol that can carry
messages over multiple hops of transport layer connections. Each transport hop,
along the route of a message, may use a different transport protocol.

It is possible to describe a route where the first hop is a TCP connection and
the second hop is also a TCP connection. Or a different route where the first
hop is bluetooth connection, the second hop is TCP connection, and the third
hop is a UDP connection and so on.

This enables end-to-end [Secure Channels](#secure-channels) over complex,
multi-hop, multi-protocol routes. It also enables en-route encrypted messages
to be stored in databases, message queues and caches for asynchronous,
end-to-end, secure communication between entities that may not be online at
the same time.

### Transports

High level Ockam protocol implementations, like Secure Channels and
Credential Exchange, are designed to remain the same regardless of how
their messages are delivered. Support for a specific transport protocol can
be plugged into the routing layer as a Transport add-on.

### Vaults

Various Ockam protocols, like secure channels, key lifecycle, credential
exchange, device enrollment etc. depend on a variety of standard
cryptographic primitives or building blocks. Depending on the environment,
these building blocks may be provided by a software implementation or a
cryptographically capable hardware component.

In order to support a variety of cryptographically capable hardware we
maintain loose coupling between our protocols and how a specific
building block is invoked in a specific hardware. This is achieved using
an abstract `Vault` interface. A concrete implementation of the `Vault`
interface is called an Ockam Vault. Over time, and with help from the Ockam
open source community, we plan to add vaults for several TEEs, TPMs, HSMs
and Secure Enclaves.

### Enterprise Integrations

Ockam protocols and libraries are designed to become a part of larger
enterprise systems and applications. To make integration easy with existing
enterprise applications, we are building add-ons that tightly integrate Ockam
with other systems like Kafka, InfluxDB and Okta that are commonly leveraged
within modern enterprise architectures.

## Get Started

We've put together a short walk through of building your first
Ockam application,
[click here to begin]("documentation/guides/rust/README.md#rust-guide").

## License

This code is licensed under the terms of the [Apache License 2.0](LICENSE).

<hr>

<p>
<a href="https://github.com/ockam-network/ockam/actions?query=workflow%3A%22Continuous+Integration%22">
<img alt="Continuous Integration"
  src="https://github.com/ockam-network/ockam/workflows/Continuous%20Integration/badge.svg">
</a>

<a href="https://www.ockam.io/learn/how-to-guides/high-performance-team/conduct/">
<img alt="Contributor Covenant"
  src="https://img.shields.io/badge/Contributor%20Covenant-v2.0%20adopted-ff69b4.svg">
</a>
