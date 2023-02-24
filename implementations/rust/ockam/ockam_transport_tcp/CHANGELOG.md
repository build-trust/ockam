# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
