# ockam_transport_core

[![crate][crate-image]][crate-link]
[![docs][docs-image]][docs-link]
[![license][license-image]][license-link]
[![discuss][discuss-image]][discuss-link]

Ockam is a library for building devices that communicate securely, privately
and trustfully with cloud services and other devices.

This crate provides the common code shared among the different Ockam's transport protocols.

Each specific protocol is then supported in its own crate. For example, the TCP protocol is supported in the `ockam_transport_tcp` crate.

Currently available transports include:

* `ockam_transport_tcp` - TCP transport
* `ockam_transport_udp` - UDP transport
* `ockam_transport_ble` - Bluetooth Low Energy Transport
* `ockam_transport_websocket` - WebSocket Transport
* `ockam_transport_uds` - Unix Domain Socket Transport


## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_transport_core = "0.96.0"
```

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_transport_core.svg
[crate-link]: https://crates.io/crates/ockam_transport_core

[docs-image]: https://docs.rs/ockam_transport_core/badge.svg
[docs-link]: https://docs.rs/ockam_transport_core

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
