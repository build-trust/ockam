# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.13.0 - 2021-08-10
### Added
- Added ockam_transport dependency
### Changed
- Refactored WebSocket error to integrate it with the ockam_transport's TransportError

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
