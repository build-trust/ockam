```yaml
title: Rust Guide
```

# Build End-to-End Encrypted Secure Channels with Ockam

In this step-by-step guide we’ll show code examples that exchange end-to-end
encrypted messages.

To protect en-route messages against eavesdropping, tampering, and forgery we
must exchange them over mutually authenticated, end-to-end encrypted secure
channels.

Ockam enables your applications to create Secure Channels over complex,
multi-hop, multi-protocol routes.

This allows end-to-end secure communication between application layer entities
that are not directly connected by simple point-to-point transport connections.
En-route encrypted messages can travel over multiple transport layer
connections and can be stored in message queues, databases or caches for
asynchronous, end-to-end protected communication between entities that may
not be online at the same time.

Let's [get started](./get-started/00-setup).
