# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.81.0 - 2023-02-24

### Changed

- Inlined the ockam_key_exchange_core crate into the ockam_core crate
- Updated dependencies

## 0.80.0 - 2023-02-09

### Changed

- Recipient returns an error instead of panicking
- Updated dependencies

## 0.79.0 - 2023-01-31

### Changed

- Updated dependencies

### Removed

- Remove unused errors from `ockam` crate

## 0.78.0 - 2023-01-18

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service
- Add support for access control for inlets&outlets

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Recover remote forwarder
- Rework access control
- Deny all for `ockam` crate
- Setup access controls for `ockam_transport_tcp`
- Setup access controls for forwarder
- Setup access controls for channels and pipes
- Make access controls a mandatory argument while creating a detached context
- Make access controls a mandatory argument while starting a worker or processor
- Improve access controls implementations
- Avoid using `Arc` on access controls in public api
- Improve `Forwarder` service access controls
- Improve `Forwarder` client access control
- Split `AccessControl` trait into `IncomingAccessControl` and `OutgoingAccessControl`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes
- Cleanup
- Disable ockam_abac in `no_std` env
- Fix `tracing` usage on no_std example project

### Removed

- Remove old credentials and signatures code

## 0.77.0 - 2022-11-08

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service
- Add support for access control for inlets&outlets

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Recover remote forwarder
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes
- Cleanup

### Removed

- Remove old credentials and signatures code

## 0.76.0 - 2022-09-21

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service
- Add support for access control for inlets&outlets

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.75.0 - 2022-09-09

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.74.0 - 2022-09-07

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.73.0 - 2022-09-05

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.72.0 - 2022-08-31

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.71.0 - 2022-08-29

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move credentials to `ockam_identity`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding
- Creation of static forwarder without heartbeats
- Creation of static forwarder at local nodes

### Removed

- Remove old credentials and signatures code

## 0.70.0 - 2022-08-17

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization
- Add `credential` module to `ockam` crate
- Add static forwarding service

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Define credential structure in ockam crate
- Non-optional `Attributes`
- Track verification status in types
- Use `IdentityChangeHistory` for verification
- Implement `CredentialExchangeWorker`
- Updated dependencies

### Fixed

- Fix forwarding service did not include onward route after forwarding

### Removed

- Remove old credentials and signatures code

## 0.69.0 - 2022-08-12

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Updated dependencies

### Removed

- Remove old credentials and signatures code

## 0.68.0 - 2022-08-04

### Added

- Support `Route` instead of `Address` for `RemoteForwarder` initialization

### Changed

- Implement attribute-based access control for message flow authorization
- Updated dependencies

### Removed

- Remove old credentials and signatures code

## 0.66.0 - 2022-07-18

### Removed

- Remove old credentials and signatures code

## 0.65.0 - 2022-07-15

### Removed

- Remove old credentials and signatures code

## 0.64.0 - 2022-07-15

### Removed

- Remove old credentials and signatures code

## 0.63.0 - 2022-06-30

### Changed

- Create worker builder for cleaner worker access control initialisation
- `Storage` -> `AuthenticatedTable`
- Identity updates
- `AuthenticatedTable` -> `AuthenticatedStorage`

## 0.61.0 - 2022-06-14

### Added

- Add `#[ockam::node]` macro attribute `access_control`

### Changed

- Implement initial access control prototype
- Refinements to initial access control prototype
- Move nodeman protocol definitions to submodule
- Move node manager service to ockam_api crate
- Create node builder for easier node initialisation

### Fixed

- Improve the usability of ockam command

## 0.60.0 - 2022-06-06

### Added

- Add basic node manager service
- Add pid query to nodeman worker

### Changed

- Implement new `Vault` serialization
- Rename new_context to new_detached
- Implement basic ockam_command config module
- Updated dependencies

## 0.59.0 - 2022-05-23

### Changed

- Change description of the ockam crate
- Updated dependencies

### Fixed

- Fix flaky transport tests

## 0.58.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.57.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.56.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.55.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.54.0 - 2022-04-25

### Added

- Add "crate" attribute to async_try_clone_derive macro

### Changed

- Updated dependencies

## 0.53.0 - 2022-04-19

### Changed

- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.52.0 - 2022-04-11

### Added

- Add docs and rename some of the request/response types in `ockam`
- Add "crate" attribute to "node" macro

### Changed

- Get rid of common `RouterMessage` in favor of transport-specific structs (ble, ws)
- Reorganize and document `ockam` crate
- Tune up some of the documentation
- Rename `mod remote_forwarder` module to `mod remote`, fix examples
- Ensure more documentation ends up in the right place
- Implement miniature `ockam` command for demo
- Re-export `DelayedEvent` from ockam crate
- Vault updates
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Fix clippy warnings

## 0.51.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.50.0 - 2022-03-28

### Added

- Add basic pipe builder usage documentation
- Support dynamic pipe handshake with behaviour handlers
- Support ordering enforcing pipes

### Changed

- Rename heartbeat to delayed event
- Sketch initial pipe2 api
- System graph initialisation builder
- Provide builder init mechanism for system handers
- Mock up pipe builder initialisation
- Allow worker system to use different message types
- Very basic pipe example with new api
- Implement basic sender resend handler
- Pipe2 dynamic pipe handshakes
- Basic delivery ack handler implementation
- Send resend notify events to delivery handler
- Update system handler trait documentation
- Run rustfmt on ockam crate
- Various clippy fixes
- Updated dependencies

### Fixed

- Fix ockam::test macro tests
- Fix ockam::node macro tests
- Imports ockam context as used in the input function
- No_std support for system builders and hooks

### Removed

- Remove stray allow(unused)

## 0.47.0 - 2022-02-22

### Added

- Add `Echoer` worker to ockam crate
- Add static forwarding support for `RemoteForwarder`

## 0.46.0 - 2022-02-08

### Changed

- Rename `Profile` -> `Identity`
- Rename crate ockam_entity -> ockam_identity
- Update crate edition to 2021

## 0.43.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.42.0 - 2021-12-13

### Added

- Add a test for full pipe behaviour stack

### Changed

- Introduce nested pipe behaviour test
- Initial ockam channel implementation
- Simplify channel creation handshake
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

### Fixed

- Fix channel channel behavior and add tests
- Clippy style update
- Update channels with no_std support

## 0.41.0 - 2021-12-06

### Changed

- Merge macro crates

### Fixed

- Change context import from ockam_node to crate
- Fix pipe test and typos

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.40.0 - 2021-11-22


### Added

- Add pipemodifier return value to behaviour stack

### Changed

- Deny warnings in ci, not local development
- Basic pipe sender implementation
- Implement static pipes
- Implement pipe sender resend logic
- Implement full pipe resend behaviour
- Move pipe tests into a separate module
- Initial ordered pipe behaviour implementation
- Enable pipe behavior stacks to be cloned
- Implement dynamic pipe handshake initialisation

### Fixed

- Enable ockam crate to use ockam_node_test_attribute
- Fix compilation for no_std environments


## v0.39.0 - 2021-11-15
### Changed
- Dependencies updated
- change `Doesnt` to `DoesNot` for enum variants

## v0.38.0 - 2021-11-08
### Changed
- handle failed fetch_intervals gracefully
- Dependencies updated

## v0.37.0 - 2021-11-01
### Changed
- Explicitly derive Message trait
- Dependencies updated

## v0.36.0 - 2021-10-26
### Changed
- Clippy improvements
- Dependencies updated

## v0.35.0 - 2021-10-25
### Added
- Expose AsyncTryClone from ockam crate.

### Changed
- Make APIs async.
- Make Stream workers async.
- Dependencies updated
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Various documentation improvements.

### Removed
- Remove protocol parser.
- Remove `None` errors from Error enums.

## v0.34.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated
## v0.33.0 - 2021-10-11

### Changed
- Dependencies updated

## v0.32.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.31.0 - 2021-09-27
### Changed
- Ockam compiles under no_std + alloc.
- Use forked version of crates core2 and serde_bare.
- Dependencies updated

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
