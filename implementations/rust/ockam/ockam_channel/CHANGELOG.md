# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.11.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Renamed SecureChannelListener callback to `completed_callback_address`
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.10.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.9.0 - 2021-05-30
### Added
- Populate key exchange changes.
### Changed
- Fix clippy issues.
- Dependency updates.

## v0.8.0 - 2021-05-17
### Added
- Entity abstraction.
### Changed
- Dependencies updated.
- Use result_message in Vault Worker.

## v0.7.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
- Documentation edits.
### Deleted

## v0.6.0 - 2021-05-03
### Changed
- Secure Channels are now created using Profiles.
- Dependencies updated.

## v0.5.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.5.0 - 2021-04-22
### Changed
- Vault struct renames.

## v0.4.0 - 2021-04-19
### Changed
- Dependencies updated.

## v0.3.1 - 2021-04-14
### Changed
- Dependencies updated.

## v0.3.0 - 2021-04-13
### Changed
- Renamed Context address functions.
- Dependencies updated.

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

