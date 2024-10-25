# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.126.0 - 2024-10-25

### Added

- Updated dependencies

## 0.125.0 - 2024-10-24

### Added

- Updated dependencies

## 0.124.0 - 2024-10-23

### Added

- Updated dependencies

## 0.123.0 - 2024-10-16

### Added

- Updated dependencies

## 0.122.0 - 2024-10-11

### Added

- Updated dependencies

## 0.121.0 - 2024-09-23

### Added

- Updated dependencies

## 0.120.0 - 2024-08-14

### Added

- Updated dependencies

## 0.119.0 - 2024-08-12

### Added

- Updated dependencies

## 0.118.0 - 2024-08-06

### Added

- Updated dependencies

## 0.117.0 - 2024-07-29

### Added

- Add the possibility to configure the default client timeout
- Adjust timeouts
- Report more detailed errors
- Updated dependencies

## 0.116.0 - 2024-07-03

### Added

- Updated dependencies

### Changed

- Use a published dependency for the patched sqlx library

## 0.115.0 - 2024-07-01

### Added

- Use the any driver for sqlx to add support for postgres
- Change tcp protocol serialization
- Optimize cbor encoding by preallocating memory
- Add secure channel padding and optimize encoding
- Updated dependencies

### Changed

- `project-member` commands, and adds the `show` command

## 0.114.0 - 2024-06-25

### Added

- Updated dependencies

## 0.113.0 - 2024-06-11

### Added

- Updated dependencies

### Fixed

- Increase test sleep duration

## 0.112.0 - 2024-05-30

### Added

- Keep secure channel even if credentials check failed
- Updated dependencies

## 0.111.0 - 2024-05-28

### Added

- Improve output of `node show` and `status` commands
- Add more log messages
- Implement updating route to the responder in the existing sc
- Introducing a variant of the secure channel which only exchange keys
- Using key exchanger in kafka secure channel map
- Add secure channel persistence
- Add secure channel persistence to kafka
- Updated dependencies

### Fixed

- Allow initial credential exchange for key exchange only

## 0.110.0 - 2024-04-30

### Added

- Updated dependencies

## 0.109.0 - 2024-04-23

### Added

- Scope some repositories to a given node name
- Updated dependencies

## 0.108.0 - 2024-04-08

### Added

- Added metadata and terminal concepts
- Updated dependencies

## 0.107.0 - 2024-04-01

### Added

- Backcompatible encoding/decoding optimizations
- Add `project-member` subcommand
- Support non-utf8 attributes in `Display` impl
- Create 3 separate credential retriever types
- Updated dependencies

## 0.106.0 - 2024-03-25

### Added

- Backcompatible encoding/decoding optimizations
- Add `project-member` subcommand
- Updated dependencies

## 0.105.0 - 2024-03-18

### Added

- Instrument more functions for secure channels
- Introduced several cpu consumption optimizations
- Updated dependencies

## 0.104.0 - 2024-02-28

### Added

- Delete `TrustContext`
- Add a unit test for deleting expired attributes
- Rename identity fields:
- Increase `MAX_ALLOWED_TIME_DRIFT` from 5 to 60 seconds
- Improve credentials management
- Address review comments
- Instrument more functions for enrollement
- Remove `clock_skew_gap` from `CachedCredentialRetriever`
- Pass the tracing context at the ockam message level
- Improve portals reliability and integration tests
- Refactor `Project`-related code
- Updated dependencies

### Changed

- Move the handling of attributes expiration date to a layer above the repository
- Separate transport messages from local messages
- Incorporate review comments

### Fixed

- Fix clippy warnings on nightly
- Close the context automatically on each test macro execution
- Increase credential duration for tests
- Fix identity attributes expiration
- Increase secure channel sleep in tests
- Store policies isolated by node and resource
- Update identity storage tests to account for orchestrator
- Avoid panicking while receiving invalid handshake message

### Removed

- Remove unused functions
- Remove some unnecessary context stops

## 0.103.0 - 2024-01-09

### Added

- Use `From` for converting errors
- Updated dependencies

## 0.102.0 - 2024-01-04

### Added

- Introduce sql data node isolation
- Updated dependencies

## 0.101.0 - 2023-12-26

### Changed

- Updated dependencies

## 0.100.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.99.0 - 2023-12-16

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Handle close and refresh credentials secure channel messages
- Persist application data in a database
- Updated dependencies

### Fixed

- Fix the passing of space name
- Fix the creation of an identity with optional name and vault

## 0.98.0 - 2023-12-15

### Changed

- Updated dependencies

## 0.97.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.96.0 - 2023-12-11

### Changed

- Updated dependencies

## 0.95.0 - 2023-12-06

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Handle close and refresh credentials secure channel messages
- Persist application data in a database
- Updated dependencies

### Fixed

- Fix the passing of space name
- Fix the creation of an identity with optional name and vault

## 0.94.0 - 2023-12-05

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Handle close and refresh credentials secure channel messages
- Persist application data in a database
- Updated dependencies

### Fixed

- Fix the passing of space name
- Fix the creation of an identity with optional name and vault

## 0.93.0 - 2023-11-23

### Changed

- Use `Identifier` as a return type in public api
- Updated dependencies

## 0.92.0 - 2023-11-17

### Changed

- Use `Identifier` as a return type in public api
- Updated dependencies

## 0.91.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.90.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.89.0 - 2023-11-02

### Changed

- Updated dependencies

## 0.88.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.87.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.86.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.85.0 - 2023-10-07

### Changed

- Improve `Vault` type-safety
- Updated dependencies

### Fixed

- Fix some credential and timeout issues

## 0.84.0 - 2023-10-05

### Changed

- Improve `Vault` type-safety
- Updated dependencies

### Fixed

- Fix some credential and timeout issues

## 0.83.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.82.0 - 2023-09-23

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.81.0 - 2023-09-22

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.80.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.79.0 - 2023-09-06

### Added

- Add `v2` module to `ockam_identity`
- Add secure channel implementation to new identity design

### Changed

- Start using `PurposeKeys` from the storage
- Updated dependencies

### Fixed

- Fix flaky stop secure channel test
- Fix new identity design tests

## 0.78.0 - 2023-06-26

### Added

- Add stop_secure_channel test

### Changed

- Improve type safety for `FlowControls`
- Hide `Spawner` vs `Producer` logic under the hood
- Make sure that ephemeral secrets are removed from memory
- Updated dependencies

### Fixed

- Extend channel test

### Removed

- Remove `FId`

## 0.77.0 - 2023-06-09

### Changed

- Make `AccessControl` optional while starting a `Worker`
- Updated dependencies

## 0.76.0 - 2023-05-26

### Changed

- Introduce a retrieve identity function returning an option
- Use identity identifiers for the creation of secure channels
- Use identity identifier for credentials
- Use an identity identifier for the authority service
- Regroup all the vault related types and traits in the same crate
- Extract the vault_aws crate
- First implementation of 3 packet exchange
- Move `FlowControls` to `Context` and make it mandatory
- Make `FlowControl` more mistake-resistant
- Improve `TCP` `::connect()` and `::listen()` outputs
- Improve `::create_secure_channel()` and `::create_secure_channel_listener()` output
- Improve `ockam_transport_tcp` registry
- Rename identity identifier from_string to from_hex for clarity
- Updated dependencies

### Fixed

- Fix compilation errors after rebasing

### Removed

- Remove the relationship between identity identifier and key id

## 0.75.0 - 2023-05-12

### Changed

- Secure channel rekey
- Updated dependencies

## 0.74.0 - 2023-05-04

### Changed

- Updated dependencies

## 0.73.0 - 2023-04-27

### Changed

- Extract identity as an entity
- Updated dependencies

## 0.72.0 - 2023-04-14

### Added

- Add trust context struct and traits
- Add trust context config and insantiate node manager with trust options

### Changed

- Implement custom get_env
- Update credential exchange worker to use trust context
- Use trust context within the creation of ockam_api secure channels
- Introduce `TrustOptions::insecure()` and `::insecure_test()`
- Improve `SecureChannelListener` `TrustOptions` for better support of consumer use case
- Simplify `TrustOptions` for outgoing negotiations
- Rename `insecure_test` -> `new`
- Rename `Sessions` -> `FlowControls`
- Rename `TrustOptions` -> `Options`
- Use `FlowControls` for `CredentialIssuer`
- Updated dependencies

### Fixed

- Fixes after tough rebase

## 0.71.0 - 2023-03-28

### Added

- Add `TrustOptions` to secure channel
- Added unit tests for the credential / credential data display instances
- Add missing serialize / deserialize instances

### Changed

- Use sessions in ockam_api
- Updated credentials example
- Make trust arguments mandatory
- Initialize the credential example with a change history and the latest key
- Modified according to review comments
- Display the date and time for a credential
- `Sessions` update
- Clean `TrustOptions` processing
- Create an authority node
- Retrieve the identity authority before creating the authority node
- Replace sessions-related `LocalInfo` with querying `Sessions`
- Updated dependencies

### Fixed

- Fix clippy warnings on test code
- Improve and extend `Sessions` tests

### Removed

- Removed type parameters exposing implementation details
- Remove the need for _arc functions
- Remove `Cancel`

## 0.70.0 - 2023-03-03

### Added

- Added a minimal authority implementation

### Changed

- Renamed authority to credential issuer
- Preload the credential issuer with attributes for alice and bob
- Moved some helper code from examples to the ockam_identity crate
- Expand credential commands
- Update secure-channel create to allow for a provided credential
- Updated dependencies

## 0.69.0 - 2023-02-24

### Added

- Added a unit test for the credential serialization

### Changed

- Move the `OneTimeCode` struct from the ockam_api crate to the ockam_identity crate
- Pre-trusted identity identifiers attributes
- Simplify the set_credentials function
- Use credential instead of credentials
- Allow the route macro to use both routes and addresses
- Updated dependencies

### Fixed

- Fix encoding of bytes on credentials and attributes
- Fixed the broken links in the rust doc

### Removed

- Remove the lifetime annotation on `Credential` and `Attributes`

## 0.68.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.67.0 - 2023-01-31

### Added

- Add tests for new encryption decryption secure channel api

### Changed

- Create `SecureChannelRegistry`
- Merge `ockam_channel` into `ockam_identity`
- Move `storage` and `registry` to `Identity`
- Rename `registry` -> `secure_channel_registry`
- Improve typing for new encrypt decrypt secure channel api
- Improve inline doc for `ockam_identity` crate
- Updated dependencies

### Fixed

- Fix `stop_secure_channel` implementation

## 0.65.0 - 2022-11-08

### Added

- Add `known_identifier` flag to secure channel command
- Add identity security tests
- Add credential exchange support to secure channel involved commands
- Add credential access control

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Update identity structure
- Switch to arch agnostic integers for secret length
- Eagerly get membership credential
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build
- Fix identity signature check
- Move async tests to using `ockam_macros::test` to prevent hanging on panic

### Removed

- Remove old credentials and signatures code

## 0.64.0 - 2022-09-21

### Added

- Add `known_identifier` flag to secure channel command
- Add identity security tests
- Add credential exchange support to secure channel involved commands
- Add credential access control

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Update identity structure
- Switch to arch agnostic integers for secret length
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build
- Fix identity signature check

### Removed

- Remove old credentials and signatures code

## 0.63.0 - 2022-09-09

### Added

- Add `known_identifier` flag to secure channel command
- Add identity security tests
- Add credential exchange support to secure channel involved commands

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Update identity structure
- Switch to arch agnostic integers for secret length
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build
- Fix identity signature check

### Removed

- Remove old credentials and signatures code

## 0.62.0 - 2022-09-07

### Added

- Add `known_identifier` flag to secure channel command
- Add identity security tests

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Update identity structure
- Switch to arch agnostic integers for secret length
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build
- Fix identity signature check

### Removed

- Remove old credentials and signatures code

## 0.61.0 - 2022-09-05

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build

### Removed

- Remove old credentials and signatures code

## 0.60.0 - 2022-08-31

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build

### Removed

- Remove old credentials and signatures code

## 0.59.0 - 2022-08-29

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Check controller's identity id when creating secure channel
- Update attributes storage structure
- Change `IdentityIdentifier` prefix to `I`
- Improve secure-channel `create` command and add `delete` command
- Create `PublicIdentity`, cleanup identity
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Make identity own a credential
- Rename `make_verified` -> `into_verified`
- Authority type improvement
- Updated dependencies

### Fixed

- Align serialisations of `IdentityIdentifier`
- Revert prefix change
- Fix no_std build

### Removed

- Remove old credentials and signatures code

## 0.58.0 - 2022-08-17

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Make `IdentityIdentifier` encodable
- Define credential structure in ockam crate
- Updated dependencies

### Removed

- Remove old credentials and signatures code

## 0.57.0 - 2022-08-12

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

### Removed

- Remove old credentials and signatures code

## 0.56.0 - 2022-08-04

### Added

- Add `known_identifier` flag to secure channel command

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

### Removed

- Remove old credentials and signatures code

## 0.54.0 - 2022-07-18

### Changed

- Updates for clippy 0.1.62

### Removed

- Remove old credentials and signatures code

## 0.53.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

### Removed

- Remove old credentials and signatures code

## 0.52.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

### Removed

- Remove old credentials and signatures code

## 0.51.0 - 2022-06-30

### Changed

- Create worker builder for cleaner worker access control initialisation
- Identity updates
- `AuthenticatedTable` -> `AttributesStorage`

## 0.49.0 - 2022-06-14

### Added

- Add `Identity` service
- Add import and export for `Contact` and `ExportedIdentity`
- Add chain verififcation where needed
- Add `#[ockam::node]` macro attribute `access_control`

### Changed

- Move ockam_identity service to ockam_api
- Implement initial access control prototype
- Refinements to initial access control prototype
- Create node builder for easier node initialisation

## 0.48.0 - 2022-06-06

### Changed

- Switch `Vault` to `String` `KeyId` instead of integer `Secret`
- Rename new_context to new_detached
- Split secure channel worker in ockam_identity crate
- Updated dependencies

## 0.47.0 - 2022-05-23

### Changed

- Updated dependencies

### Fixed

- Fix flaky transport tests

## 0.46.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.45.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.44.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.43.0 - 2022-04-25

### Added

- Add "crate" attribute to async_try_clone_derive macro

### Changed

- Updated dependencies

## 0.42.0 - 2022-04-19

### Changed

- Clean up ockam_core import paths
- Update broken tests
- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam_identity
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.41.0 - 2022-04-11

### Added

- Add timeout to `SecureChannel` creation

### Changed

- Reorganize and document `ockam` crate
- Don't re-export `hex` or `hashbrown` from `ockam_core`
- Implement miniature `ockam` command for demo
- Vault updates
- Make `Identity` trait immutable
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Fix clippy warnings

## 0.40.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.39.0 - 2022-03-28

### Changed

- Friendlify api for `ockam_core::access_control`
- Friendlify api for `ockam_core::vault::key_id_vault`
- Updated dependencies

## 0.36.0 - 2022-02-08

### Changed

- Rename crate ockam_entity -> ockam_identity
- Async compat updates to identity and vault
- Update crate edition to 2021

## 0.35.0 - 2022-01-31

### Changed

- Document `JSON` `TokenLeaseManager` client

## 0.34.0 - 2022-01-26

### Added

- Add `TrustPublicKeyPolicy`

### Changed

- Make trust policy take &mut self
- Ssh secure channel echoer cli

## 0.33.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

### Fixed

- Fix credentials build failure

## 0.32.0 - 2021-12-13

### Added

- Add access control
- Add ockam_core/bls feature and small fixes

### Changed

- Vault updates
- Implement add key for entity
- Update entity structure
- Update `LocalInfo` logic
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

### Removed

- Remove stale ref to `KeyAttributes`

## 0.31.0 - 2021-12-06

### Added

- Add logging to responder side of entity secure channel

### Changed

- Merge macro crates
- Make secure channel print warning when destination is not available during decryption

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.30.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development
- Run `cargo clippy --fix`


## v0.29.0 - 2021-11-15
### Changed
- Dependencies updated
- change `Doesnt` to `DoesNot` for enum variants

## v0.28.0 - 2021-11-08
### Changed
- replace `AsyncTryClone` trait impls with `#[derive(AsyncTryClone)]` wherever applicable
- Dependencies updated

## v0.27.0 - 2021-11-01
### Changed
- Explicitly derive Message trait
- Dependencies updated

## v0.26.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.25.0 - 2021-10-25
### Changed
- Dependencies updated
- Various documentation improvements.
- Make APIs async.
- Make async-trait crate used through ockam_core.
- Replace instances of `&Vec<T>` with `&[T]`.
- Simplified feature usage.
- Move as many things as possible into a workspace.

### Removed
- Remove SecureChannels trait.
- Remove `None` errors from Error enums.

## v0.24.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
### Changed
- Make credentials optional (disabled by default)
- Use ockam_core::compat::mutex instead of cortex_m::interrupt::*
- Dependencies updated
- Move `Handle` to ockam_node

## v0.23.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.22.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.21.0 - 2021-09-27
### Changed
- Use forked version of crates core2 and serde_bare.
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.20.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.19.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.18.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.17.0 - 2021-09-03
### Added
- Lease Manager client and JSON protocol
### Changed
- Dependencies updated.

## v0.16.0 - 2021-08-30
### Added
- Created ockam_transport_core crate for generic transport code
### Changed
- Dependencies updated.

## v0.15.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.14.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
- Introduce Signature Vault type.
### Changed
- Dependencies updated.

## v0.13.0 - 2021-08-09
### Changed
- Ignore error while stopping secure channel listener.
- Dependencies updated.

## v0.12.0 - 2021-08-03
### Added
- Added a simple Entity builder.
### Changed
- Refactored entity secure channel workers.
- Dependencies updated.

## v0.11.0 - 2021-07-29
### Changed
- Clarify trust policy names.
- Fix creation of secure channel with known entity.
- Dependencies updated.
- Rename trust policies to more descriptive names.

## v0.10.0 - 2021-07-26
### Added
- Add credential protocol and implementation.
- Add add_credential and get_credential to Holder trait.
- Add reveal_attributes and credential_type macros.
- Add get_secure_channel_participant_id function.
- Add convenient creation functions to trust_policy_worker.
### Changed
- Entity create function now takes an optional id.
- Rename Credential to BbsCredential to avoid naming collision.
- Dependencies updated.

## v0.9.0 - 2021-07-19
### Added
- `credential_attribute_values` macro
- `credential_type` macro
### Changed
- Dependencies updated.
- Re-enable trust policies for secure channels, post refactor.

## v0.8.0 - 2021-07-12
### Added
- New `from_external` function to `ProfileIdentifier`, for creating identifiers from serialized forms.

### Changed
- Dependencies updated.
- Secure channel creation no longer panics when used with an entity.
- Move signing key to Profile change events.
- Entity Worker `get_contact` response changed to correct type.

## v0.7.0 - 2021-07-06
### Added
- Credential APIs based on Entities.
- `check_origin` function for `LocalMessage`.
### Changed
- Dependencies updated.

## v0.6.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.

## v0.5.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
- Added `UnknownChannelMsgOrigin` error.
### Changed
- Renamed SecureChannelListener callback to `completed_callback_address`
- Make the ProfileChannelListener `listener_address` required.
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.4.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.3.0 - 2021-05-30
### Added
- Entity abstraction.
- Trust policy abstraction and IdentityTrustPolicy implementation.

### Changed
- Dependency updates.
- Fix clippy issues.

## v0.2.0 - 2021-05-17
- Documentation and meta-information fixes.

## v0.1.0 - 2021-05-17

- Initial release.
