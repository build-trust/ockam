# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.30.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.29.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.28.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.27.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.26.0 - 2021-08-30
### Added
- Implementation of the forwarding service.
### Changed
- Dependencies updated.

## v0.25.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.24.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
### Changed
- Dependencies updated.

## v0.23.1 - 2021-08-09
### Changed
- Updated documentation.

## v0.23.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.22.0 - 2021-08-03
### Added
- Added a simple generator for unique, human-readable identifiers.
### Changed
- Fixed log message in stream producer.
- Refactored entity secure channel workers.
- Moved location of stream message fetch polling.
- Dependencies updated.

## v0.21.0 - 2021-07-29
### Changed
- Refactored streams code for clarity & ergonomics.
- Dependencies updated.

## v0.20.0 - 2021-07-26
### Added
- Add threshold BLS signing.
- Add hex as a public exported crate to ockam crate.
### Changed
- Update remote_forwarder to be able to use arbitrary address instead of socket_addr.
- Dependencies updated.

## v0.19.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.18.0 - 2021-07-12
### Added
- Utility for sending asynchronous delayed messages.
- Stream protocol initial API and implementation.
- Stream Worker implementation.
- BLS signature stub.
- New `from_external` function to `ProfileIdentifier`, for creating identifiers from serialized forms.
- Service builder for Ockam Transport implementations.
- Stream API example.
- New Builder function to `Stream` that configures the client_id for the Stream consumer.
- Monotonic id generator for ockam crate internals.
- Expose blocking and non-blocking delay functions.
- Basic publishing Worker.

### Changed
- Dependencies updated.
- Return an error instead of panicking when a protocol parser fails.
- Improve logging in Worker relay.
- Move signing key to change events
- Incoming messages now have access to stream routing information.
- Secure channel creation no longer panics when used with an entity.
- get_contact Entity Eorker response type changed to correct type.
- Bring `stream_service` and `index_service` names in line with Hub defaults.
- Bring stream protocol definitions in line with the latest definition.
- Make stream and index service addresses configurable.
- Save updated index after successful message pull.
- Support Message `return_route` via bi-directional Streams.
- Update index as messages are retrieved from Stream.
- Isolate and expose bi-directional stream names.
- Improve delayed event API.
- Allow protocol parser fragment to communicate success


## v0.17.0 - 2021-07-06
### Added
- Type for `BLS` secrets.
### Changed
- Dependencies updated.

## v0.16.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Dependencies updated.

## v0.15.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.14.0 - 2021-06-14
### Added
- `route!` macro to simplify construction of message routes.
### Changed
- Dependencies updated.

## v0.13.0 - 2021-05-30
### Added
- Entity abstraction.
- Trust Policy abstraction and IdentityTrustPolicy policy implementation.

### Changed
- Fix clippy issues.
- Dependency updates.

## v0.12.0 - 2021-05-17
### Added
- Entity abstraction.
- Modular and configurable protocol parser.
### Changed
- Dependencies updated.
- Remove dynamic dispatch in UserParser.
- Updated documentation.


## v0.11.0 - 2021-05-10
### Added
- Traits for `Profile`.
### Changed
- Dependencies updated.
- Renamed `async_worker` to `worker`.
- Documentation edits.

## v0.10.0 - 2021-05-03
### Changed
- Vault creation is now sync.
- Dependencies updated.

## v0.9.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.9.0 - 2021-04-22
### Changed
- Crate dependency reorganization.
- Vault struct renames.

## v0.8.0 - 2021-04-14
### Changed
- Build system and test fixes.
- Dependencies updated.
- Replaced RemoteMailbox with RemoteForwarder.

## v0.7.0 - 2021-04-14
### Changed
- Build system and test fixes.
- Dependencies updated.

## v0.6.0 - 2021-04-13
### Changed
- Dependencies updated.
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
- Dependencies updated.
- Improved error handling.

## v0.1.0 - 2021-02-04
### Added

- This document and other meta-information documents.
