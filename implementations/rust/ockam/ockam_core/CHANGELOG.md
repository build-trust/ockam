# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.124.0 - 2025-01-20

### Added

- Rewrite `ockam_node`
- Updated dependencies

### Changed

- Rename cloud module to orchestrator

## 0.123.0 - 2025-01-09

### Added

- Introduce env variables to adjust transport performance
- Updated dependencies

## 0.122.0 - 2024-12-04

### Added

- Avoiding memory fragmentation by reducing allocations
- Updated dependencies

## 0.121.0 - 2024-11-27

### Added

- Encoding and allocation optimizations for privileged portals
- Updated dependencies

## 0.120.0 - 2024-11-12

### Added

- Updated dependencies

### Fixed

- In portals, check identity change only for remote packets
- Error chain is kept in ockam_command crate

## 0.119.0 - 2024-10-23

### Added

- Updated dependencies

## 0.118.0 - 2024-10-15

### Added

- Updated dependencies

## 0.117.0 - 2024-09-23

### Added

- Added `TLS` inlet support
- Implement influxdb token lessor service
- Refactor influxdb api client to better handle error responses
- Updated dependencies

## 0.116.0 - 2024-08-14

### Added

- Added the possibility to encrypt specific fields in a kafka `JSON` record
- Updated dependencies

## 0.115.0 - 2024-08-12

### Added

- Updated dependencies

## 0.114.0 - 2024-08-06

### Added

- Updated dependencies

## 0.113.0 - 2024-07-29

### Added

- Updated dependencies

## 0.112.0 - 2024-07-01

### Added

- Add `FIXME` to `ockam_core`
- Change tcp protocol serialization
- Optimize cbor encoding by preallocating memory
- Updated dependencies

### Changed

- `project-member` commands, and adds the `show` command
- `kafka-*` commands

### Fixed

- Account for `minicbor` length calculation bug

## 0.111.0 - 2024-06-25

### Added

- Updated dependencies

## 0.110.0 - 2024-06-11

### Added

- Updated dependencies

### Changed

- Bump opentelemetry-appender-tracing from 0.3.0 to 0.4.0

## 0.109.0 - 2024-05-24

### Added

- Improve output of `node show` and `status` commands
- Provide the name of the environment variable when it cannot be decoded
- Add flow controls query
- Updated dependencies

### Changed

- Upgrade the rust version to 1.77

## 0.108.0 - 2024-04-30

### Added

- Improve output of `node show` command
- Switch to `aws-lc-rs` library for encryption/decryption
- Updated dependencies

## 0.107.0 - 2024-04-23

### Added

- Updated dependencies

## 0.106.0 - 2024-04-12

### Added

- Added metadata and terminal concepts
- Updated dependencies

### Changed

- Move terminal code from command to api

## 0.105.0 - 2024-04-01

### Added

- Backcompatible encoding/decoding optimizations
- Add one second cache for incoming and outgoing access control
- Reply to v1 transport messages with v1 transport messages
- Enable the tracing context on the rust side
- Create 3 separate credential retriever types
- Updated dependencies

### Fixed

- Decode a transport message even without a tracing_context field
- Only emit v1 messages

## 0.104.0 - 2024-03-25

### Added

- Backcompatible encoding/decoding optimizations
- Add one second cache for incoming and outgoing access control
- Updated dependencies

## 0.103.0 - 2024-03-18

### Added

- Added manual tests to measure latency
- Propagating the errors from api clients to the command
- Start a new trace when receiving a transport message
- Start a new trace before sending a transport message
- Introduced several cpu consumption optimizations
- Updated dependencies

## 0.102.0 - 2024-02-26

### Added

- Add opentelemetry tracing and logging support
- Improve ockam project ticket, ockam project enroll ux output, help, logs, errors
- Delete `TrustContext`
- Improve credentials management
- Address review comments
- Pass the tracing context at the ockam message level
- Add policies for resource types
- Updated dependencies

### Changed

- Separate transport messages from local messages
- Enable tracing by default
- Incorporate review comments

### Fixed

- Put the tracing context field under a compilation flag

## 0.101.0 - 2024-01-09

### Added

- Make authority issued credentials ttl configurable
- Updated dependencies

## 0.100.0 - 2024-01-04

### Added

- Updated dependencies

## 0.99.0 - 2023-12-26

### Changed

- Updated dependencies

## 0.98.0 - 2023-12-16

### Changed

- Slim down the node manager worker(s_ch)
- Updated dependencies

## 0.97.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.96.0 - 2023-12-11

### Changed

- Slim down the node manager worker(s_ch)
- Updated dependencies

## 0.95.0 - 2023-12-06

### Changed

- Updated dependencies

## 0.94.0 - 2023-12-05

### Changed

- Updated dependencies

## 0.93.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.92.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.91.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.90.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.89.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.88.0 - 2023-10-07

### Changed

- Move the controller address to the node manager
- Use better names for request / response headers
- Use a more precise interface for the subscriptions trait
- Use a secure client to enroll
- Use the authority client to enroll
- Package all reply / response methods into a client
- Updated dependencies

### Fixed

- Fix the no_std task

### Removed

- Remove an unnecessary pattern match
- Remove the unused tag feature

## 0.87.0 - 2023-10-05

### Changed

- Move the controller address to the node manager
- Use better names for request / response headers
- Use a more precise interface for the subscriptions trait
- Use a secure client to enroll
- Use the authority client to enroll
- Package all reply / response methods into a client
- Updated dependencies

### Fixed

- Fix the no_std task

### Removed

- Remove an unnecessary pattern match
- Remove the unused tag feature

## 0.86.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.85.0 - 2023-09-13

### Changed

- Extract the output of request results from the rpc code
- Updated dependencies

## 0.84.0 - 2023-09-06

### Added

- Add popup window to tcp-outlet/service creation

### Changed

- Change some response functions
- Updated dependencies

### Fixed

- Fix the cbor annotations for non-borrowed data

### Removed

- Removed api lifetimes to access node manager operations directly

## 0.83.0 - 2023-06-26

### Changed

- Extract a full state machine for the secure channel handshake
- Update ockam api services error responses to using a struct
- Updated dependencies

### Fixed

- Fix compat imports
- Include compat box for no-std

### Removed

- Remove verbose decode logic, add inner errors

## 0.82.0 - 2023-06-09

### Changed

- Updated dependencies

### Removed

- Delete `LocalOnwardOnly` and `LocalSourceOnly`

## 0.81.0 - 2023-05-26

### Changed

- Regroup all the vault related types and traits in the same crate
- First implementation of 3 packet exchange
- Updated dependencies

## 0.80.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Automate the creation and update of readmes
- Updated dependencies

### Fixed

- Fix `cargo doc` warnings

## 0.79.0 - 2023-04-27

### Changed

- Extract identity as an entity
- Improve outputs of tcp outlet, inlet and relay
- Allow kafka reconnection when project connection goes down
- Updated dependencies

## 0.78.0 - 2023-04-14

### Changed

- Serialize keys using base64 while keeping back-compatibility
- Implement custom get_env
- Compiled the env code for no_std
- Rename `Sessions` -> `FlowControls`
- Updated dependencies

### Removed

- Remove unnecessary `pub extern crate async_trait`

## 0.77.0 - 2023-03-28

### Added

- Add sessions to `ockam_core`
- Address sessions pr comments
- Add `src_addr` to `Routed<M>`

### Changed

- Derive `Debug` for no_std `RwLock`
- `Sessions` update
- Replace sessions-related `LocalInfo` with querying `Sessions`
- Updated dependencies

### Removed

- Removed type parameters exposing implementation details

## 0.76.0 - 2023-03-03

### Changed

- Updated dependencies

## 0.75.0 - 2023-02-24

### Changed

- Inlined the ockam_key_exchange_core crate into the ockam_core crate
- Split cddl schema files & merge when cbor api validation is needed
- Updated dependencies

## 0.74.0 - 2023-02-09

### Changed

- Recipient returns an error instead of panicking
- Updated dependencies

## 0.73.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.71.0 - 2022-11-08

### Added

- Add policy command

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Change echo worker to accept any message
- Avoid recursion when processing policies
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.70.0 - 2022-09-21

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Change echo worker to accept any message
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.69.0 - 2022-09-09

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.68.0 - 2022-09-07

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.67.0 - 2022-09-05

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.66.0 - 2022-08-31

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Updated dependencies

### Fixed

- Fix schema validation
- Move schema.cddl into ockam_core/src

## 0.65.0 - 2022-08-29

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Make it easier to write commands' api req/res handlers
- Check controller's identity id when creating secure channel
- Crud operation for project enrollers
- Replace signer with verifier
- Derive `Default` for `Id` and `Error`
- Updated dependencies

### Fixed

- Fix schema validation

## 0.64.0 - 2022-08-17

### Changed

- Use generic attributes in credential
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Updated dependencies

## 0.63.0 - 2022-08-12

### Changed

- Use generic attributes in credential
- Updated dependencies

## 0.62.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.57.0 - 2022-06-30

### Changed

- `Storage` -> `AuthenticatedTable`
- `AuthenticatedTable` -> `AuthenticatedStorage`

## 0.56.0 - 2022-06-14

### Changed

- Implement initial access control prototype
- Refinements to initial access control prototype

## 0.55.0 - 2022-06-06

### Added

- Add ockam_api_nodes
- Add simple `Vault` service

### Changed

- Switch `Vault` to `String` `KeyId` instead of integer `Secret`
- Implement new `Vault` serialization
- Move `TypeTag` to `ockam_core`
- Partially add cbor support to `ockam_core/vault`
- Updated dependencies

### Removed

- Remove `AsRef` from `PublicKey` to avoid confusion

## 0.54.0 - 2022-05-09

### Changed

- Rename github organization to build-trust
- Updated dependencies

## 0.53.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.52.0 - 2022-04-19

### Changed

- Introduce error type
- Build error mapping for various crates
- Clean up ockam_core import paths
- Update broken tests
- Move allow and deny utils to ockam_core root
- Rename error2 to error
- Updated dependencies

### Fixed

- Update `compat::sync::Mutex` to return `Result` instead of `Option`
- Fix error module lints
- Errors: fix ockam_core
- Errors: fix ockam_vault
- Errors: fix ockam
- Fix various clippy and rustfmt lints

### Removed

- Remove ockam_node errors and add new util module
- Remove traits module from ockam_core
- Remove thiserror as it does not support no_std

## 0.51.0 - 2022-04-11

### Changed

- Get rid of common `RouterMessage` in favor of transport-specific structs (ble, ws)
- Make `ockam_core::Error` derive `Eq` and `PartialEq`
- Don't re-export `hex` or `hashbrown` from `ockam_core`
- Tune up some of the documentation
- Ensure more documentation ends up in the right place
- Implement miniature `ockam` command for demo
- Vault updates
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Fix clippy warnings

## 0.50.0 - 2022-03-28

### Added

- Add examples for all public functions in `ockam_core`
- Add tests for `ockam_core`

### Changed

- Edit `ockam_core` documentation for typos, clarity and consistency
- Move `traits` module into its own file
- Move `ockam_core::println_no_std` to `ockam_core::compat::println`
- Friendlify api for `ockam_core::access_control`
- Friendlify api for `ockam_core::routing::address`
- Friendlify api for `ockam_core::vault::key_id_vault`
- `TODO` return `Result<&Address>` from `ockam_core::Route.recipient()`
- Implement basic sender resend handler
- Various clippy fixes
- Updated dependencies

### Removed

- Remove unused type `ockam_core::ResultMessage`

## 0.47.0 - 2022-02-22

### Added

- Add `From<(u8, String>)` implementation for `Address`

## 0.46.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.43.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0
- Use the tracing crate for logging on no_std

## 0.42.0 - 2021-12-13

### Added

- Add access control
- Add ockam_core/bls feature and small fixes

### Changed

- Update `LocalInfo` logic
- Initial ockam channel implementation
- Simplify channel creation handshake
- Move ockam_vault_core crate into ockam_core

## 0.41.0 - 2021-12-06

### Added

- Add `take_payload` to `Routed`

### Changed

- Merge macro crates

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.40.0 - 2021-11-22


### Added

- Add route prepend mechanism and test

### Changed

- Deny warnings in ci, not local development


## v0.39.0 - 2021-11-15
### Changed
- fix `no_std` breakage
- Dependencies updated

## v0.38.0 - 2021-11-08
### Added
- add proc macro to auto derive `AsyncTryClone` trait
### Changed
- replace `AsyncTryClone` trait impls with `#[derive(AsyncTryClone)]` wherever applicable
- replaced tokio::try_join with futures_util::try_join
- Dependencies updated

## v0.37.0 - 2021-11-01
### Changed
- Explicitly derive Message trait
- Dependencies updated

## v0.36.0 - 2021-10-25
### Added
- Add generic AsyncTryClone implementation for structs with Clone.
### Changed
- Make async-trait crate used through ockam_core.
- Replace instances of `&Vec<T>` with `&[T]`.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated

## v0.35.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
### Changed
- Only use cortex_m_semihosting on arm platforms
- Dependencies updated

## v0.34.0 - 2021-10-11
### Added
- Implement From<Iterator> and Into<Iterator> for AddressSet
- Implement FromStr for Address
### Changed
- Dependencies updated

## v0.33.0 - 2021-10-04
### Added
- Implement AsyncTryClone for VaultSync
### Changed
- Dependencies updated

## v0.32.0 - 2021-09-27
### Changed
- Add ockam_core::compat::task.
- Use forked version of crates core2 and serde_bare.
- Use main core2 repo.
- Ockam compiles under no_std + alloc.
- thread_rng state does not advance with repeated instantiations.
- Dependencies updated

## v0.31.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.30.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.29.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.28.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.27.0 - 2021-08-30
### Added
- Processor trait.
### Changed
- Dependencies updated.

## v0.26.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.25.0 - 2021-08-16
### Changed
- Dependencies updated.

## v0.24.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.23.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.22.0 - 2021-07-29
### Changed
- Dependencies updated.
### Deleted
- Remove service builder from ockam_core.

## v0.21.0 - 2021-07-26
### Added
- Add support for multiple accept addresses for router.
### Changed
- Dependencies updated.

## v0.20.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.19.0 - 2021-07-12
### Added
- Stream Worker implementation and creation.
- Stream API example.
- Service builder for Ockam Transport implementations.

### Changed
- Dependencies updated.
- Bring Streams implementation up to date with the newly introduced LocalMessage type.
- Return an error instead of panicking when a protocol parser fails.
- Incoming messages now have access to stream routing information.

## v0.18.0 - 2021-07-06
### Changed
- Dependencies updated.

## v0.17.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.
- Dependencies updated.

## v0.16.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.15.0 - 2021-06-14
### Added
- `route!` macro to simplify construction of message routes.

## v0.14.0 - 2021-05-30
### Added
### Changed
- Dependency updates.
- Fix clippy issues.

## v0.13.0 - 2021-05-17
### Added
- Modular and configurable protocol parser.
- result_message type.
### Changed
- Dependencies updated.
- Worker shutdown is now async.

## v0.12.0 - 2021-05-10
### Added
### Changed
- Renamed `async_worker` to `worker`.
### Deleted

## v0.11.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.11.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.11.0 - 2021-04-19
### Changed
- Dependencies updated.
- Updated a routing error internal domain code.

## v0.10.0 - 2021-04-14

### Changed
- Improved debug printability of addresses.
- Improved TCP transport initialisation.

## v0.9.0 - 2021-04-13
### Changed
- Dependencies updated.
- Renamed Context address functions.
- Improved printability of messages and payloads.

## v0.8.0 - 2021-04-12
### Added
- `Any` message type added to ockam_core crate.
### Changed
- Dependencies updated.
- `Routed` message wrapper function APIs renamed.
- `Passthrough` type renamed to `Any`.
- `msg_addr` moved from `Context` to `Routed`.
- `Context` address renamed to `primary_address`.
- Transport message fields renamed.


## v0.7.0 - 2021-04-05
### Added
- Expose onward route information to user workers.
- Random address generation.

### Changed
- Switch payload encoding from bincode to bare.
- Split forwarding examples into two binaries.
- Dependencies updated.
- Rename route fields in transport message.

### Removed
- RemoteMailbox has been moved to the `ockam` crate.


## v0.6.0 - 2021-03-22
### Added
- Routable message abstraction.
- Builder for Routable messages.
- Route metadata for messages.
- Generic transport message.

### Changed

- Dependencies updated.
- Core dependencies, such as hashbrown and hex have been re-exported from this crate.

## v0.5.0 - 2021-03-04
### Added

- Support multiple addresses per worker.

## v0.4.0 - 2021-03-03
### Added

- Auto-implementation of the `Message` trait for certain types.

### Changed

- The `Worker` trait and its methods are now async.
- Dependencies updated.

## v0.3.0 - 2021-02-16
### Added

- Explicit `alloc`, `no_std`, and `std` features.
- Generalized `Address` implementation.
- Global crate lib facade wrapper around `std` and `core` re-exports, for cross-feature compatibility.
- Message trait base implementation.

### Changed

- Dependencies updated.
- Improved documentation.



## v0.2.0 - 2021-02-04
### Added

-  Runs Worker `initialize` function when the Worker is started.
-  Uses `From` trait in place of `Into` in Node error
-  Create Worker example

### Removed
-  Worker Builder

### Changed

-  Moved Worker and Address types to this crate.
-  Renamed executor commands to messages

## v0.1.0 - 2021-01-30
### Added

- `Error` - an error type that can be returned is both `std` and `no_std` modes.
- `Result` - a result type that can be returned is both `std` and `no_std` modes.
