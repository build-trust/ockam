# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.67.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.66.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.65.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.63.0 - 2022-11-08

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Codespell implementations/rust/
- Updated dependencies

## 0.62.0 - 2022-09-21

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.61.0 - 2022-09-09

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.60.0 - 2022-09-07

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.59.0 - 2022-09-05

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.58.0 - 2022-08-31

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.57.0 - 2022-08-29

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.56.0 - 2022-08-17

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.55.0 - 2022-08-12

### Changed

- Updates for clippy 0.1.62
- Cleanup ockam test macro
- Updated dependencies

## 0.54.0 - 2022-08-04

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.52.0 - 2022-07-18

### Changed

- Updates for clippy 0.1.62

## 0.51.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.50.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.48.0 - 2022-06-17

### Fixed

- Ignore flaky test on websocket crate

## 0.46.0 - 2022-06-06

### Changed

- Rename new_context to new_detached
- Updated dependencies

## 0.45.0 - 2022-05-23

### Changed

- Return socket address when starting a transport listener
- Code block and imports
- Updated dependencies

### Fixed

- Fix flaky transport tests

## 0.44.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.43.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.42.0 - 2022-04-25

### Added

- Add documentation for all public functions
- Add examples to transport.rs

### Changed

- Initialize stream on worker's initialize method
- Updated dependencies

### Removed

- Remove unnecessary error conversion

## 0.41.0 - 2022-04-19

### Changed

- Clean up ockam_core import paths
- Update broken tests
- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam_transport_websocket
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.40.0 - 2022-04-11

### Changed

- Get rid of common `RouterMessage` in favor of transport-specific structs (ble, ws)
- Implement miniature `ockam` command for demo
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Fix clippy warnings

## 0.39.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.38.0 - 2022-03-28

### Added

- Add basic test for ws transport

### Changed

- Ws transport
- Updated dependencies

## 0.35.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.32.0 - 2022-01-10

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

### Removed

- Delete the ockam_node_no_std crate

## 0.31.0 - 2021-12-13

### Changed

- Updated dependencies

## 0.30.0 - 2021-12-06

### Changed

- Make transport errors start from 1

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.29.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development


## v0.28.0 - 2021-11-15
### Changed
- change `WebSocket` address type constant from 2 to 3
- Dependencies updated

## v0.27.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.26.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.25.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.24.0 - 2021-10-25
### Changed
- Move as many things as possible into a workspace.
- Dependencies updated

## v0.23.0 - 2021-10-18
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated

## v0.22.0 - 2021-10-11
### Changed
- Dependencies updated
- Replace Into with AsRef

## v0.21.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.20.0 - 2021-09-27
### Changed
- Use forked version of crates core2 and serde_bare.
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.19.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.18.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.17.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.16.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.15.0 - 2021-08-30
### Added
- Created ockam_transport_core crate for generic transport code
### Changed
- Migrate TcpError to TransportError
- Dependencies updated.

## v0.14.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.13.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
### Changed
- Dependencies updated.

## v0.12.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.11.0 - 2021-08-03
### Changed
- Refactored entity secure channel workers.
- Dependencies updated.

## v0.10.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.9.0 - 2021-07-26
### Changed
- Dependencies updated.
- Updated to latest internal routing APIs

## v0.8.0 - 2021-07-19
### Added
### Changed
- Dependencies updated.
- Worker pair address parameters are now in the correct order.
- Remove Context borrowing in Websocket Transport.

## v0.7.0 - 2021-07-12
### Changed
- Dependencies updated.

## v0.6.0 - 2021-07-06
### Changed
- Dependencies updated.

## v0.5.0 - 2021-06-30
### Changed
- Fix clippy warnings.
- Dependencies updated.

## v0.4.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Standardize all Ockam crates to use the same version of `tokio`.
- Dependencies updated.

## v0.3.0 - 2021-06-14
### Changed
- Dependencies updated
- Pinned all versions in `Cargo.toml`

## v0.2.0 - 2021-05-30
### Added
### Changed
- Dependency updates.
- Fix websocket errors.
- Fix websocket transport type.

## v0.1.0 - 2021-04-21

### Added

- First version of a WebSocket transport implementation, based on the TCP transport implementation
