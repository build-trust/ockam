```yaml
title: Get Started
```

# Build an End-to-End Encrypted Application.

To protect en-route messages against eavesdropping, tampering, and forgery …
we usually need a cryptographic secure channel protocol.

Most message transport protocols support some way to establish a secure
channel. However, such secure channel protocols have traditionally been tightly
coupled to their corresponding transport protocols. Their security guarantees
are limited by the length and duration of a single transport layer connection.

This constraint, often leads to application architectures that violate the
foundational security principle of least privilege … exposing applications to
a vulnerability and liability surface that is a lot bigger than it needs to be.

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
the second hop is a different TCP connection. Or a route where the first
hop is bluetooth connection, the second hop is TCP connection, and the third
hop is a UDP connection and so on.

This enables end-to-end Secure Channels over complex, multi-hop, multi-protocol
routes. It also enables en-route encrypted messages to be stored in databases,
message queues and caches for asynchronous, end-to-end, secure communication
between entities that may not be online at the same time.

## Get started.

The Ockam Rust libraries make it easy to build such end-to-end encrypted
applications, so let's build one together.

<div style="display: none; visibility: hidden;">
<a href="./00-setup">00. Setup</a>
</div>
