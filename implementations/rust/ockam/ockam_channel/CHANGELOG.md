# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.3.0 - 2021-04-13
### Changed
- Renamed Context address functions.
- Updated dependencies.

## v0.2.0 - 2021-04-12
### Added

### Changed
- `Routed` message wrapper function APIs renamed.
- Refactored Node Context API.
- Decouple channel addresses on initiator and responder sides.
- Rename secure_channel worker_address to address
- Add convenience Into<Address> generics for channel initializations.
- `msg_addr` moved from `Context` to `Routed`.
- `Context` address renamed to `primary_address`.
- Transport message fields renamed.


### Deleted
- Removed channel message wrappers.

## v0.1.0 - 2021-04-05

- Initial release.

