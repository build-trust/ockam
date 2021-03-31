# Ockam Implementations

Ockam protocols can be implemented in various languages. Our current focus is [Rust](rust) and
[Elixir](elixir) but over time we will support many languages.

Below is the maturity status of various Ockam features in Rust and Elixir.

## Features

| Implementation | Feature              | Maturity                      | Description                                |
|:-------------- |----------------------|:------------------------------|--------------------------------------------|
| Rust           | Node - Standard      | ![preview][preview]           | Run workers                                |
| Rust           | Node - No Standard   | ![planned][planned]           | Run workers in embedded environments       |
| Rust           | Workers              | ![preview][preview]           | Concurrent actors with addresses           |
| Rust           | Routing              | ![preview][preview]           | Multi-hop application layer routing        |
| Rust           | Transports           | ![preview][preview]           | Pluggable transports                       |
| Rust           | Transport - TCP      | ![preview][preview]           | Add-on for Ockam routing over TCP          |
| Rust           | Secure Channels      | ![experimental][experimental] | Encrypted channels over Ockam routing      |
| Rust           | Key Agreement - XX   | ![experimental][experimental] | A mutually authenticated key agreement     |
| Rust           | Key Agreement - X3DH | ![experimental][experimental] | An asynchronous key agreement              |
| Rust           | Vaults               | ![experimental][experimental] | Pluggable cryptographic hardware           |
| Rust           | Vault - Software     | ![experimental][experimental] | Add-on for a pure software vault           |
| Rust           | Vault - ATECC608     | ![experimental][experimental] | Add-on for a Microchip ATECC608 vault      |
| Rust           | Entities             | ![planned][planned]           | Simple API and encapsulation               |
| Rust           | Profiles             | ![experimental][experimental] | Identity profiles for entities             |
| Rust           | Credentials          | ![experimental][experimental] | Credentials with selective disclosure      |
| Rust           | Credentials - BBS+   | ![experimental][experimental] | BBS+ signatures for Credentials            |
| Rust           | Credentials - PS     | ![planned][planned]           | PS signatures for Credentials              |
| Elixir         | Node                 | ![experimental][experimental] | Run workers                                |
| Elixir         | Workers              | ![experimental][experimental] | Concurrent actors with addresses           |
| Elixir         | Routing              | ![experimental][experimental] | Multi-hop application layer routing        |
| Elixir         | Transports           | ![experimental][experimental] | Pluggable transports                       |
| Elixir         | Transport - TCP      | ![experimental][experimental] | Add-on for Ockam routing over TCP          |
| Elixir         | Secure Channels      | ![experimental][experimental] | Encrypted channels over Ockam routing      |
| Elixir         | Key Agreement - XX   | ![experimental][experimental] | A mutually authenticated key agreement     |
| Elixir         | Key Agreement - X3DH | ![planned][planned]           | An asynchronous key agreement              |
| Elixir         | Vaults               | ![experimental][experimental] | Pluggable cryptographic hardware           |
| Elixir         | Vault - Software     | ![experimental][experimental] | Add-on for a pure software vault           |
| Elixir         | Entities             | ![planned][planned]           | Simple API and encapsulation               |
| Elixir         | Profiles             | ![planned][planned]           | Identity profiles for entities             |
| Elixir         | Credentials          | ![planned][planned]           | Credentials with selective disclosure      |
| Elixir         | Credentials - BBS+   | ![planned][planned]           | BBS+ signatures for Credentials            |
| Elixir         | Credentials - PS     | ![planned][planned]           | PS signatures for Credentials              |

[planned]: https://img.shields.io/badge/Status-Planned-EEEEEE.svg?style=flat-square
[experimental]: https://img.shields.io/badge/Status-Experimenal-FFD932.svg?style=flat-square
[preview]: https://img.shields.io/badge/Status-Preview-6BE3CF.svg?style=flat-square
[stable]: https://img.shields.io/badge/Status-Stable-81D553.svg?style=flat-square
[depricated]: https://img.shields.io/badge/Status-Stable-EC6D57.svg?style=flat-square
