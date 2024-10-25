# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.129.0 - 2024-10-25

### Added

- Updated dependencies

## 0.128.0 - 2024-10-24

### Added

- Check capabilities before using ebpf portals
- Updated dependencies

## 0.127.0 - 2024-10-23

### Added

- Updated dependencies

### Changed

- Bump aya from 0.12.0 to 0.13.0

## 0.126.0 - 2024-10-16

### Added

- `eBPF` portal updates:
- Add ebpf portal bats tests
- Updated dependencies

## 0.125.0 - 2024-10-15

### Added

- Use more monolith structure for ebpf portals
- Updated dependencies

### Changed

- Bump rustls-native-certs from 0.7.3 to 0.8.0

## 0.124.0 - 2024-09-23

### Added

- Added `TLS` inlet support
- Implementation of reliable `TCP` portals
- Unload ebpfs on `ockam reset`
- Updated dependencies

## 0.123.0 - 2024-08-14

### Added

- Heavy kafka refactoring, moved portal interceptor from `api` to `tcp` crate
- Kafka cleanups
- Updated dependencies

## 0.122.0 - 2024-08-12

### Added

- Updated dependencies

## 0.121.0 - 2024-08-06

### Added

- Updated dependencies

## 0.120.0 - 2024-07-29

### Added

- Improve transport imports
- Implicitly resolve outlet addresses during connection
- Remove sync operations
- Updated dependencies

## 0.119.0 - 2024-07-03

### Added

- Updated dependencies

## 0.118.0 - 2024-07-01

### Added

- Improve transport imports
- Add possibility to pause `TCP` inlets
- Change tcp protocol serialization
- Add secure channel padding and optimize encoding
- Updated dependencies

### Fixed

- Account for `minicbor` length calculation bug

## 0.117.0 - 2024-06-25

### Added

- Updated dependencies

## 0.116.0 - 2024-06-11

### Added

- Updated dependencies

## 0.115.0 - 2024-05-30

### Added

- Updated dependencies

## 0.114.0 - 2024-05-24

### Added

- Implement updating route to the outlet in the existing inlet
- Updated dependencies

## 0.113.0 - 2024-04-30

### Added

- Switch from `native-tls` to `tokio-rustls` for outlet tls
- Updated dependencies

## 0.112.0 - 2024-04-23

### Added

- Support https for outlets
- Make the api for creating outlets more flexible
- Updated dependencies

## 0.111.0 - 2024-04-12

### Added

- Added metadata and terminal concepts
- Fix portals protocol
- Updated dependencies

## 0.110.0 - 2024-04-01

### Added

- Backcompatible encoding/decoding optimizations
- Updated dependencies

### Fixed

- Fix routing and flow control for local kafka outlets
- Decode a transport message even without a tracing_context field

## 0.109.0 - 2024-03-25

### Added

- Backcompatible encoding/decoding optimizations
- Updated dependencies

### Fixed

- Fix routing and flow control for local kafka outlets

## 0.108.0 - 2024-03-18

### Added

- Add spans for portals
- Instrument the tcp portal
- Introduced several cpu consumption optimizations
- Updated dependencies

## 0.107.0 - 2024-02-28

### Added

- Add opentelemetry tracing and logging support
- Add sleep to tcp tests
- Address review comments
- Send pong to the inlet only after the outlet connected
- Pass the tracing context at the ockam message level
- Improve portals reliability and integration tests
- Updated dependencies

### Changed

- Separate transport messages from local messages

### Fixed

- Improve tcp resolution tests
- Close the context automatically on each test macro execution
- Disable portal packet counter field
- Race condition when payload is sent before the pong message

## 0.106.0 - 2024-01-09

### Added

- Updated dependencies

## 0.105.0 - 2024-01-04

### Added

- Updated dependencies

## 0.104.0 - 2023-12-26

### Changed

- Updated dependencies

## 0.103.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.102.0 - 2023-12-16

### Changed

- Updated dependencies

## 0.101.0 - 2023-12-15

### Changed

- Updated dependencies

## 0.100.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.99.0 - 2023-12-11

### Changed

- Updated dependencies

## 0.98.0 - 2023-12-06

### Changed

- Updated dependencies

## 0.97.0 - 2023-12-05

### Changed

- Updated dependencies

## 0.96.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.95.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.94.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.93.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.92.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.91.0 - 2023-10-07

### Changed

- Updated dependencies

## 0.90.0 - 2023-10-05

### Changed

- Updated dependencies

## 0.89.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.88.0 - 2023-09-23

### Changed

- Updated dependencies

## 0.87.0 - 2023-09-22

### Changed

- Updated dependencies

## 0.86.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.85.0 - 2023-09-06

### Changed

- Improve tcp disconnect api
- Updated dependencies

### Fixed

- Use the outlet socket address to search for the outlet status

## 0.84.0 - 2023-06-26

### Changed

- Improve type safety for `FlowControls`
- Hide `Spawner` vs `Producer` logic under the hood
- Updated dependencies

## 0.83.0 - 2023-06-09

### Changed

- Make `AccessControl` optional while starting a `Worker`
- Updated dependencies

## 0.82.0 - 2023-05-26

### Changed

- Move `FlowControls` to `Context` and make it mandatory
- Make `FlowControl` more mistake-resistant
- Improve `TCP` `::connect()` and `::listen()` outputs
- Improve `SecureChannel` and `TCP` options
- Updated dependencies

## 0.81.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Automate the creation and update of readmes
- Updated dependencies

### Fixed

- Fix `cargo doc` warnings

## 0.80.0 - 2023-04-27

### Changed

- Updated dependencies

### Fixed

- Resolve transport addresses as a separate step

## 0.79.0 - 2023-04-14

### Changed

- Introduce `TrustOptions::insecure()` and `::insecure_test()`
- Simplify `TrustOptions` for outgoing negotiations
- Rename `insecure_test` -> `new`
- Rename `Sessions` -> `FlowControls`
- Rename `TrustOptions` -> `Options`
- Disable `FlowControl` for loopback tcp connections and listeners
- Updated dependencies

## 0.78.0 - 2023-03-28

### Added

- Add `TrustOptions` to `ockam_transport_tcp`. refactor connection creation
- Address sessions pr comments
- Add `Sessions` support to receiving messages in `ockam_node`

### Changed

- Use sessions in ockam_api
- Make trust arguments mandatory
- `Sessions` update
- Clean `TrustOptions` processing
- Replace sessions-related `LocalInfo` with querying `Sessions`
- Updated dependencies

## 0.77.0 - 2023-03-03

### Added

- Add `TCP::disconnect` and `TCP::stop_listener`
- Add `TCP` disconnection and stop listener tests
- Add small sleep after `tcp.stop_listener()` in test

### Changed

- Rework `TCP`
- Make `TCP::listen()` return worker `Address`
- Move `TCP` connection process out of `TcpSendWorker`
- Updated dependencies

### Fixed

- Improve `TCP` tests

## 0.76.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.75.0 - 2023-02-09

### Changed

- Make the portal message struct public
- Updated dependencies

## 0.74.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.72.0 - 2022-11-08

### Added

- Add support for access control for inlets&outlets
- Add tcp keepalive and remove tcp heartbeat

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.71.0 - 2022-09-21

### Added

- Add support for access control for inlets&outlets

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.70.0 - 2022-09-09

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.69.0 - 2022-09-07

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.68.0 - 2022-09-05

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.67.0 - 2022-08-31

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.66.0 - 2022-08-29

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.65.0 - 2022-08-17

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.64.0 - 2022-08-12

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.63.0 - 2022-08-04

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.61.0 - 2022-07-18

### Changed

- Updates for clippy 0.1.62

## 0.60.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.59.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.58.0 - 2022-06-30

### Changed

- Create worker builder for cleaner worker access control initialisation

## 0.56.0 - 2022-06-14

### Added

- Add `#[ockam::node]` macro attribute `access_control`

### Changed

- Implement initial access control prototype
- Refinements to initial access control prototype

## 0.55.0 - 2022-06-06

### Changed

- Rename new_context to new_detached
- Updated dependencies

### Removed

- Remove messaging cycle from `TCP Portal`

## 0.54.0 - 2022-05-23

### Changed

- Return socket address when starting a transport listener
- Code block and imports
- Updated dependencies

### Fixed

- Fix flaky transport tests
- Fix tcp router race condition

## 0.53.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.52.0 - 2022-05-05

### Added

- Add delay to tcp portal

### Changed

- Log and ignore error while sending disconnect from portal
- Use 10kb buffer for tcp portal
- Updated dependencies

### Fixed

- Fix tcp receiver heartbeat handling

## 0.51.0 - 2022-05-04

### Changed

- Updated dependencies

### Fixed

- Reduce `MAX_PAYLOAD_SIZE` back to 256

## 0.50.0 - 2022-05-04

### Changed

- Increase buffer size for tcp portal
- Updated dependencies

## 0.49.0 - 2022-04-25

### Added

- Add tests for `ockam_transport_tcp`
- Add documentation for `ockam_transport_tcp`
- Add "crate" attribute to async_try_clone_derive macro

### Changed

- Friendlify code organisation of `ockam_transport_tcp::TcpRouter`
- Friendlify code organisation of `ockam_transport_tcp::TcpRouterHandle`
- Friendlify code organisation of `ockam_transport_tcp::TcpListenProcessor`
- Friendlify code organisation of `ockam_transport_tcp::TcpSendWorker`
- Move `TcpRouter` into its own file
- Updated dependencies

### Fixed

- Fixes #2630

## 0.48.0 - 2022-04-19

### Changed

- Clean up ockam_core import paths
- Run rustfmt
- Updated dependencies

### Fixed

- Errors: fix ockam_transport_tcp
- Fix various clippy and rustfmt lints

## 0.47.0 - 2022-04-11

### Added

- Add `Tcp` disconnect test

### Changed

- Implement tcp disconnection
- Implement manual disconnection for `Tcp`
- Implemented tcp connection to already connected ip under different hostname
- Updated dependencies

### Fixed

- Fix clippy warnings

### Removed

- Remove outdated tcprouter docs

## 0.46.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.45.0 - 2022-03-28

### Changed

- Rename heartbeat to delayed event
- Updated dependencies

## 0.42.0 - 2022-02-22

### Fixed

- Fix message type in tcp sender
- Fix `TcpTransport` initialization race condition
- Fix tcp send_receive test

## 0.41.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.38.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

### Removed

- Delete the ockam_node_no_std crate

## 0.37.0 - 2021-12-13

### Added

- Add basic portal test
- Add tcp heartbeats

### Changed

- Upgrade portals flow
- Adjust portals delays to avoid race conditions
- Stop tcp worker on heartbeat failure

### Fixed

- Fix clippy warnings

## 0.36.0 - 2021-12-06

### Changed

- Merge macro crates

### Fixed

- Use `write_all` instead of `write` for tcp

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.35.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development


## v0.34.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.33.0 - 2021-11-08
### Changed
- Dependencies updated
- use cluster mechanism for tcp transport workers
- replace `AsyncTryClone` trait impls with `#[derive(AsyncTryClone)]` wherever applicable

## v0.32.0 - 2021-11-01
### Changed
- explicitly derive message trait
- replace std::sleep with tokio in tcp test
- fix tcp lazy connection ordering
- Dependencies updated

## v0.31.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.30.0 - 2021-10-25
### Changed
- Implement AsyncTryClone for TcpTransport.
- Make async-trait crate used through ockam_core.
- Replace instances of `&Vec<T>` with `&[T]`.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated

### Removed
- Remove block_future from TCP.

## v0.29.0 - 2021-10-18

### Changed
- Only use cortex_m_semihosting on arm platforms
- Dependencies updated

## v0.28.0 - 2021-10-11
### Added
- TcpTransport stop_outlet
- Generalized string argument types in TcpTransport
### Changed
- Dependencies updated

## v0.27.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.26.0 - 2021-09-27
### Changed
- Use forked version of crates core2 and serde_bare.
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.25.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.24.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.23.0 - 2021-09-13
### Added
### Changed
- Dependencies updated.

## v0.22.0 - 2021-09-03
### Changed
- Fix Portals interaction logic
- Dependencies updated.

## v0.21.0 - 2021-08-30
### Added
- Created ockam_transport_core crate for generic transport code
### Changed
- Migrate TcpError to TransportError
- Dependencies updated.
- Implement Processors for TCP and Inlet listeners and receivers.

## v0.20.0 - 2021-08-23
### Added
- Add TCP Portals.
### Changed
- Replace std:: modules with core:: and alternate implementations
- Derive clone for TcpTransport
- Dependencies updated.

## v0.19.0 - 2021-08-16
### Changed
- Dependencies updated.

## v0.18.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.17.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.16.0 - 2021-07-29
### Changed
- Dependencies updated.
- Refactor of TCP Transport code

## v0.15.0 - 2021-07-26
### Added
- Add lazy TCP connections.
- Add DNS hostname resolution to TCP transport.
### Changed
- Dependencies updated.

## v0.14.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-12
### Added
- Service builder for Ockam Transport implementations.
### Changed
- Dependencies updated.
## v0.12.0 - 2021-07-06
### Added
### Changed
- Dependencies updated.
- Avoid borrowing `Context`.

## v0.11.0 - 2021-06-30
### Added
### Changed
- Fix clippy warnings.
- Dependencies updated.

## v0.10.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Standardize all Ockam crates to use the same version of `tokio`.
- Dependencies updated.

## v0.9.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.8.0 - 2021-05-30
### Added
### Changed
- Dependencies updated.
- Replace TCP Transport type with const.

## v0.7.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.

## v0.6.3 - 2021-05-10
### Added
### Changed
- Documentation edits.
### Deleted

## v0.6.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.6.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.6.0 - 2021-04-22
### Changed
- Crate dependency reorganization.

## v0.5.1 - 2021-04-19
### Changed
- Dependencies updated.

## v0.5.0 - 2021-04-15
### Changed
- Improved TCP Transport API.

## v0.4.0 - 2021-04-14
### Added
- Added dead_code lint.
- Enabled multi-hop routes via domain specific routers.

### Changed
- Improved TCP transport initialisation.
- Improved the flow of the TCP Transport API.
- Dependencies updated.
- Build system and test fixes.


## v0.3.0 - 2021-04-13
### Changed
- Improved TCP echo example.
- Gracefully handle TCP connection failures.
- Improved printability of messages and payloads.
- Improved logging for dropped TCP connections.
- `msg_addr` moved from `Context` to `Routed`.
- Dependencies updated.
- Renamed Context address functions.
- Refactored Node Context API.
- Renamed `Routed` message wrapper function API.
- Simplified TCP Worker API for most common use cases.
- Take TCP addresses as strings and parse internally.

## v0.2.0 - 2021-03-22
### Added
- Route metadata wrapper type.
- New implementations of TCP Router and TCP Listener.

### Changed

- Dependencies updated.
- Split TCP worker into two parts: sender & receiver.


## v0.1.0 - 2021-02-10
### Added

- `Connection` - a trait that represents transport connections.
- `Listener` - a trait that represents transport connection listeners.
- `TcpConnection` - a TCP implementation of Connection.
- `TcpListener` - a TCP implementation of Listener.
