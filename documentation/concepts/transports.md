```yaml
title: Transports
order: 3
```

# Transports: Ockam Add-Ons for Transport Protocols

![Ockam](./assets/ockam-features-routing.png)

Ockam's Application Layer Routing decouples our suite of cryptographic
protocols, like secure channels, key lifecycle, credential exchange, enrollment
etc. from the underlying transport protocols.

Our high level protocols are designed to remain the same regardless of how
their messages are delivered, this allows us to establish end-to-end trust
between application layer entities and remove the trust and privilege that is
typically placed in network intermediaries and infrastructure.

New protocol support can be easily added using simple transport protocol
add-ons that plug into Ockam routing.
