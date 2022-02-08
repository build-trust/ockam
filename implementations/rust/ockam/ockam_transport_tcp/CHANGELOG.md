# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

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
- Stop tcp worker on hearbeat failure

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
