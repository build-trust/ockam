# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
