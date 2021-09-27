# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
