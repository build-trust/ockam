# ockam_transport_udp

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
ockam_transport_udp = "0.5.0"
```

## Test

In `ockam_transport_udp` directory, ran `cargo test`.

## Examples

In `ockam_transport_udp` directory, run an echo server
with command `cargo run --exapmle echo_server`

Then, run a client that sends a hello message to the server
with command `cargo run --example client`

## License

This code is licensed under the terms of the [Apache License 2.0][license-link].

[main-ockam-crate-link]: https://crates.io/crates/ockam

[crate-image]: https://img.shields.io/crates/v/ockam_transport_tcp.svg
[crate-link]: https://crates.io/crates/ockam_transport_tcp

[docs-image]: https://docs.rs/ockam_transport_tcp/badge.svg
[docs-link]: https://docs.rs/ockam_transport_tcp

[license-image]: https://img.shields.io/badge/License-Apache%202.0-green.svg
[license-link]: https://github.com/build-trust/ockam/blob/HEAD/LICENSE

[discuss-image]: https://img.shields.io/badge/Discuss-Github%20Discussions-ff70b4.svg
[discuss-link]: https://github.com/build-trust/ockam/discussions
