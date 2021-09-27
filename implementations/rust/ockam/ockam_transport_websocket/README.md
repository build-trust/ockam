# ockam_transport_websocket

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides a WebSocket Transport for Ockam's Routing Protocol.

The Routing Protocol decouples Ockam's suite of cryptographic protocols,
like secure channels, key lifecycle, credential exchange, enrollment etc. from
the underlying transport protocols. This allows applications to establish
end-to-end trust between entities.

WebSocket is one possible transport for Routing Protocol messages, over time there
will be more transport implementations.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_transport_websocket = "0.19.0"
```

This crate requires the rust standard library `"std"`.

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_transport_websocket.svg
[crate-link]: https://crates.io/crates/ockam_transport_websocket

[docs-image]: https://docs.rs/ockam_transport_websocket/badge.svg
[docs-link]: https://docs.rs/ockam_transport_websocket

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/ockam-network/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/ockam-network/ockam/discussions
