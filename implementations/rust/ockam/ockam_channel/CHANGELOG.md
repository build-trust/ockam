# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.24.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.23.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.22.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.21.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.20.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
### Changed
- Dependencies updated.

## v0.19.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.18.0 - 2021-08-03
### Changed
- Refactored entity secure channel workers.
- Moved location of stream message fetch polling.
- Dependencies updated.

## v0.17.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.16.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.15.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.14.0 - 2021-07-12
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-06
### Added
- Type for `BLS` secrets.
### Changed
- Dependencies updated.

## v0.12.0 - 2021-06-30
### Changed
- Dependencies updated.

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

