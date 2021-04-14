# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.7.0 - 2021-04-14
### Changed
- Build system and test fixes.
- Updated dependencies.

## v0.6.0 - 2021-04-13
### Changed
- Updated dependencies.
- Renamed Context address functions.

## v0.5.0 - 2021-04-12
### Added
- `Any` message type for untyped worker messages.

### Changed
- `Routed` message wrapper function APIs renamed.
- `Passthrough` type renamed to `Any`.
- `msg_addr` moved from `Context` to `Routed`.
- `Context` address renamed to `primary_address`.
- Transport message fields renamed.
- RemoteMailbox function renames.


## v0.4.2 - 2021-04-05
### Changed
- Dependency updates.

## v0.4.1 - 2021-03-31
### Changed
- Updated documentation.

## v0.4.0 - 2021-03-23
### Added

- Unified message type for router implementations.
- Route metadata wrapper type.

### Changed
- Dependency updates.

## v0.3.0 - 2021-03-04
### Added
- Lease API.
- Credential API.
- Profile authentication.
- Profile key rotation.

### Changed
- Contact and Profile APIs file locations have been reorganized.
- Use new trait names from `ockam_vault_core`.
- Dependency updates.
- Renamed `authentication_factor` to `authentication_proof`.
- Minor Profile API improvements.

### Removed
- Removed explicit `async_trait` user dependency.
- Contacts has been removed in favor of profiles.

## v0.2.0 - 2021-02-17
### Added
- Contact and Profile APIs.
- Profile Changes API.

### Changed
- Updated dependencies.
- Improved error handling.

## v0.1.0 - 2021-02-04
### Added

- This document and other meta-information documents.

## v0.2.0 - 2021-02-05
### Added

- Profile implementation.
