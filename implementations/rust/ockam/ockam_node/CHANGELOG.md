# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.131.0 - 2024-10-25

### Added

- Updated dependencies

## 0.130.0 - 2024-10-24

### Added

- Updated dependencies

## 0.129.0 - 2024-10-23

### Added

- Added optional watchdog for tokio blocking tasks
- Switching to sqlite wal mode for better concurrency
- Updated dependencies

## 0.128.0 - 2024-10-16

### Added

- Updated dependencies

## 0.127.0 - 2024-10-15

### Added

- Updated dependencies

## 0.126.0 - 2024-09-23

### Added

- Updated dependencies

## 0.125.0 - 2024-08-14

### Added

- Updated dependencies

## 0.124.0 - 2024-08-12

### Added

- Updated dependencies

## 0.123.0 - 2024-08-06

### Added

- Updated dependencies

## 0.122.0 - 2024-07-29

### Added

- Converted socket addresses to hostnames in command
- Adjust timeouts
- Report more detailed errors
- Stop a previous medic before starting a new one
- Updated dependencies

### Fixed

- Allow the creation of a sqlite database with a relative path

## 0.121.0 - 2024-07-03

### Added

- Updated dependencies

### Changed

- Use a published dependency for the patched sqlx library

## 0.120.0 - 2024-07-01

### Added

- Use the any driver for sqlx to add support for postgres
- Change tcp protocol serialization
- Optimize cbor encoding by preallocating memory
- Updated dependencies

## 0.119.0 - 2024-06-25

### Added

- Updated dependencies

## 0.118.0 - 2024-06-11

### Added

- Set sql statement log level to trace
- Updated dependencies

## 0.117.0 - 2024-05-30

### Added

- Make `default` vault reuse `SqlxDatabase` instance
- Updated dependencies

## 0.116.0 - 2024-05-28

### Added

- Add the possibility to use boolean expressions for policy expressions
- Add flow controls query
- Added abac rules to kafka inlet and oulet
- Updated dependencies

### Changed

- Upgrade the rust version to 1.77

## 0.115.0 - 2024-04-30

### Added

- Switch to `aws-lc-rs` library for encryption/decryption
- Command `aws-lc` library via a feature
- Updated dependencies

## 0.114.0 - 2024-04-23

### Added

- Scope some repositories to a given node name
- Updated dependencies

### Fixed

- Shutdown processor and worker on init fail

## 0.113.0 - 2024-04-12

### Added

- Added metadata and terminal concepts
- Updated dependencies

## 0.112.0 - 2024-04-01

### Added

- Backcompatible encoding/decoding optimizations
- Reply to v1 transport messages with v1 transport messages
- Enable the tracing context on the rust side
- Updated dependencies

## 0.111.0 - 2024-03-25

### Added

- Backcompatible encoding/decoding optimizations
- Updated dependencies

## 0.110.0 - 2024-03-18

### Added

- Start a new trace before sending a transport message
- Introduced several cpu consumption optimizations
- Updated dependencies

### Removed

- Remove resources when deleting a node

## 0.109.0 - 2024-02-28

### Added

- Add support for additional kafka addons
- Add opentelemetry tracing and logging support
- Implement `Default` for `ockam_node::compat::Mutex`
- Delete `TrustContext`
- Add application errors
- Implement `sleep_long` sleep accounting for device sleep
- Address review comments
- Add policy migration that removes `trust_context_id`
- Pass the tracing context at the ockam message level
- Add policies for resource types
- Improve portals reliability and integration tests
- Rework migrations
- Updated dependencies

### Changed

- Separate transport messages from local messages

### Fixed

- Fix sqlx migration
- Close the context automatically on each test macro execution
- Store policies isolated by node and resource
- Set the proper span id on the propagated tracing context

### Removed

- Remove panic if onward route in invalid
- Remove some unnecessary context stops

## 0.108.0 - 2024-01-09

### Added

- Catch execution panics and exit the process
- Log panic messages
- Updated dependencies

### Fixed

- Fix worker and processor relay start order

## 0.107.0 - 2024-01-04

### Added

- Updated dependencies

## 0.106.0 - 2023-12-26

### Changed

- Close unneeded tcp connections in various clients
- Updated dependencies

## 0.105.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.104.0 - 2023-12-16

### Changed

- Updated dependencies

## 0.103.0 - 2023-12-15

### Changed

- Updated dependencies

## 0.102.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.101.0 - 2023-12-11

### Changed

- Updated dependencies

## 0.100.0 - 2023-12-06

### Changed

- Updated dependencies

## 0.99.0 - 2023-12-05

### Changed

- Updated dependencies

## 0.98.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.97.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.96.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.95.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.94.0 - 2023-10-18

### Changed

- Updated dependencies

### Fixed

- Fix typos
- Handle error returned by the `main` function

## 0.93.0 - 2023-10-07

### Changed

- Use better names for request / response headers
- Move the secure client close to secure channels
- Adjust the code after rebase
- Package all reply / response methods into a client
- Use the client in the background node
- Updated dependencies

### Fixed

- Fix some clippy warnings
- Fix a no std warning

### Removed

- Remove two parameters from requests to the controller

## 0.92.0 - 2023-10-05

### Changed

- Use better names for request / response headers
- Move the secure client close to secure channels
- Adjust the code after rebase
- Package all reply / response methods into a client
- Use the client in the background node
- Updated dependencies

### Fixed

- Fix some clippy warnings
- Fix a no std warning

### Removed

- Remove two parameters from requests to the controller

## 0.91.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.90.0 - 2023-09-23

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.89.0 - 2023-09-22

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.88.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.87.0 - 2023-09-06

### Changed

- Introduce an app state holding a context
- Updated dependencies

### Removed

- Removed api lifetimes to access node manager operations directly

## 0.86.0 - 2023-06-26

### Added

- Add a debugger feature for the ockam_identity crate

### Changed

- Rebase on develop
- Updated dependencies

## 0.85.0 - 2023-06-09

### Changed

- Clean `FlowControls` resources on `Address` stop
- Make `AccessControl` optional while starting a `Worker`
- Improve `ProcessorBuilder`. make `AccessControl` optional while starting a `Processor`
- Updated dependencies

## 0.84.0 - 2023-05-26

### Changed

- First implementation of 3 packet exchange
- Present credentials during secure channel exchange by default
- Migrate the identities configuration
- Initialize the default node outside of the command run impl
- Move `FlowControls` to `Context` and make it mandatory
- Updated dependencies

### Fixed

- Fix minor typos

## 0.83.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Updated dependencies

## 0.82.0 - 2023-04-27

### Changed

- Extract identity as an entity
- Simplify an instantiation with default
- Use a rw lock instead of a mutex to store transports
- Updated dependencies

### Fixed

- Resolve transport addresses as a separate step

## 0.81.0 - 2023-04-14

### Changed

- Implement custom get_env
- Update `ockam_node/api`
- Updated dependencies

## 0.80.0 - 2023-03-28

### Added

- Add `ctx.receive_timeout` test
- Add `Sessions` support to receiving messages in `ockam_node`

### Changed

- Set the log level as debug instead of warning when getting messages
- Updated dependencies

### Removed

- Remove `Cancel`

## 0.79.0 - 2023-03-03

### Added

- Add getter for `WorkerBuilder` and `ProcessorBuilder`

### Changed

- Updated dependencies

## 0.78.0 - 2023-02-24

### Added

- Added a debug instance for context

### Changed

- Split cddl schema files & merge when cbor api validation is needed
- Updated dependencies

### Fixed

- Commands shows concise errors with a more human-readable format
- Fixed the broken links in the rust doc
- Update project readiness check to include authority

## 0.77.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.76.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.74.0 - 2022-11-08

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Minor refactors to commands/api error handling
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.73.0 - 2022-09-21

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Minor refactors to commands/api error handling
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.72.0 - 2022-09-09

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Minor refactors to commands/api error handling
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.71.0 - 2022-09-07

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Minor refactors to commands/api error handling
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.70.0 - 2022-09-05

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.69.0 - 2022-08-31

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.68.0 - 2022-08-29

### Added

- Add support for panic handling in test macro
- Add timeout and node exists check to `message send` command

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Check identity during credentials exchange
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors
- Fix schema validation

### Removed

- Remove `block_future`

## 0.67.0 - 2022-08-17

### Added

- Add support for panic handling in test macro

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Move api structs to `ockam_core`
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors

### Removed

- Remove `block_future`

## 0.66.0 - 2022-08-12

### Added

- Add support for panic handling in test macro

### Changed

- Implement attribute-based access control for message flow authorization
- Cleanup ockam test macro
- Updated dependencies

### Fixed

- Use runtime handles
- Fix mispointed link
- Check if address already exists before creating workers/processors

### Removed

- Remove `block_future`

## 0.65.0 - 2022-08-04

### Changed

- Implement attribute-based access control for message flow authorization
- Updated dependencies

### Fixed

- Use runtime handles

### Removed

- Remove `block_future`

## 0.60.0 - 2022-06-30

### Changed

- Create worker builder for cleaner worker access control initialisation
- `Storage` -> `AuthenticatedTable`
- `AuthenticatedTable` -> `AuthenticatedStorage`
- Partially implemented node watchdog

### Fixed

- Making ockam_node a bit less spammy on debug

## 0.59.0 - 2022-06-17

### Changed

- Disable metrics by default

## 0.58.0 - 2022-06-14

### Added

- Add metrics output to trace logs
- Add `#[ockam::node]` macro attribute `access_control`

### Changed

- Generate simple csv metrics report
- Collect router and (ockam) worker metrics
- Rename context metrics field and add better docs
- Implement initial access control prototype
- Refinements to initial access control prototype
- Move node manager service to ockam_api crate
- Create node builder for easier node initialisation

### Fixed

- Clean up router metrics code lints
- Gate metrics code behind metrics feature

## 0.57.0 - 2022-06-06

### Added

- Add an async_drop mechanism for bare context drop
- Add dedicated channel types for ockam_node, switch back to bounded channels
- Add timeout function to context that takes duration

### Changed

- Attempt to cut down memory usage via context drop
- Rename new_context to new_detached
- Updated dependencies

### Fixed

- Fix address de-allocation issues for bare contexts

## 0.56.0 - 2022-05-23

### Changed

- Updated dependencies

### Fixed

- Fix flaky transport tests
- Enable `SpanTrace` capture during tracing registration

## 0.55.0 - 2022-05-09

### Changed

- Rename github organization to build-trust
- Updated dependencies

## 0.54.0 - 2022-05-05

### Changed

- Move to unbound channel
- Updated dependencies

### Fixed

- Fix `ProcessorRelay`
- Fix typo

## 0.53.0 - 2022-04-25

### Added

- Add send_and_receive method to context

### Changed

- Updated dependencies

### Fixed

- Do not drop control channel sender when processing stopworker message

## 0.52.0 - 2022-04-19

### Changed

- Introduce error type
- Build error mapping for various crates
- Clean up ockam_core import paths
- Run rustfmt
- Rename error2 to error
- Rebuilding the ockam_node error types
- Updated dependencies

### Fixed

- Errors: fix ockam_core
- Fixing lints
- Fix various clippy and rustfmt lints

### Removed

- Remove ockam_node errors and add new util module
- Remove thiserror as it does not support no_std

## 0.51.0 - 2022-04-11

### Changed

- Get rid of common `RouterMessage` in favor of transport-specific structs (ble, ws)
- Make `ockam_node::error` module public
- Reorganize and document `ockam` crate
- Don't re-export `hex` or `hashbrown` from `ockam_core`
- Implement miniature `ockam` command for demo
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Ensure that the command supports `OCKAM_LOG`
- Fix clippy warnings

## 0.50.0 - 2022-04-04

### Changed

- Updated dependencies

### Fixed

- Use serde_bare to prepend length to payload when missing

## 0.49.0 - 2022-03-28

### Added

- Add context usage documentation and update api docs

### Changed

- Rename compat.rs to compat/mod.rs
- Update cancel documentation
- Rename heartbeat to delayed event
- Improve executor documentation
- Friendlify api for `ockam_core::access_control`
- Updated dependencies

### Removed

- Delete ockam_node context handle

## 0.46.0 - 2022-02-22

### Added

- Add `Heartbeat` to ockam_node

### Changed

- Implement worker ready status api

## 0.45.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.44.0 - 2022-01-31

### Added

- Add compat mod to ockam_node with async `Mutex` and `RwLock`
- Add unsafe async `RwLock` implementation

## 0.43.0 - 2022-01-26

### Fixed

- Update thread_local due to rustsec issue
- Fix error handling in channel, cargo update

## 0.42.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0
- Use the tracing crate for logging on no_std

### Fixed

- Protect processor relays against accidental async executor deadlock

### Removed

- Delete the ockam_node_no_std crate

## 0.41.0 - 2021-12-13

### Added

- Add access control

### Changed

- Updated dependencies

## 0.40.0 - 2021-12-06

### Changed

- Improve ockam_node logging, fix typos
- Move and improve ockam_node tests

### Fixed

- Partial fix of `WorkerRelay` ctrl_rx usage
- Prevent `Router` from stopping when message handling error encountered

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.39.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development
- Improve node router and mailbox logging
- Reduce router cluster command verbosity

### Fixed

- Fix crash crash scenario on fallback shutdown strategy


## v0.38.0 - 2021-11-15
### Changed
- change `Doesnt` to `DoesNot` for enum variants
- Dependencies updated

## v0.37.0 - 2021-11-08
### Changed
- Dependencies updated
- reduce log spam from start operations
- implement shutdown abortion on timeout
- implement graceful stop mechanism in ockam node
- introduce node shutdown type
- remove double-nested tests module from ockam_node
- pull router address state out into separate module
- implement command rejection during node shutdown
- break router out into separate module tree
- simplify processor stop mechanism
- simplify mailbox architecture

## v0.36.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.35.0 - 2021-10-26
### Changed
- Clippy improvements
- Dependencies updated

## v0.34.0 - 2021-10-25
### Changed
- Make handle async only.
- Make async-trait crate used through ockam_core.
- Replace instances of `&Vec<T>` with `&[T]`.
- Simplified feature usage.
- Dependencies updated
### Removed
- Remove `None` errors from Error enums.

## v0.33.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
### Changed
- Various improvements to ockam_executor
- Only use cortex_m_semihosting on arm platforms
- Use ockam_core::compat::mutex instead of cortex_m::interrupt::*
- Move `Handle` to ockam_node
- Dependencies updated

## v0.32.0 - 2021-10-11
### Added
- Introduce Context send_from_address_impl
- Implement From<Iterator> for AddressSet
### Changed
- Extract private implementations and Into wrappers
- Forward error from executor::execute()
- Dependencies updated

## v0.31.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.24.0 - 2021-09-27
### Changed
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.29.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.28.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.27.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.26.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.25.0 - 2021-08-30
### Added
- Processor implementation for ockam_node
### Changed
- Dependencies updated.

## v0.24.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.23.0 - 2021-08-16
### Changed
- Dependencies updated.

## v0.22.0 - 2021-08-09
### Changed
- Move sender out of Mailbox
- Avoid RelayMessage cloning
- Make mailbox field private
- Restructure router internals
- Dependencies updated.
### Deleted
- Remove Drop implementation for Context

## v0.21.0 - 2021-08-03
### Changed
- Raised default message polling timeout to 30 seconds.
- Dependencies updated.

## v0.20.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.19.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.18.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.17.0 - 2021-07-12
### Added
- Stream API example.
- Utility for sending asynchronous delayed messages.

### Changed
- Dependencies updated.
- Improve logging in Worker relay.

## v0.16.0 - 2021-07-06
### Changed
- Dependencies updated.

## v0.15.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.

## v0.14.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Standardize all Ockam crates to use the same version of `tokio`.
- TransportMessage constructor has been extended to use recent routing changes.
- Dependencies updated.

## v0.13.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.12.0 - 2021-05-30
### Added
### Changed
- Dependencies updated.

## v0.11.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
- Worker shutdown is now async.
### Deleted


## v0.10.0 - 2021-05-10
### Added
### Changed
- Context receive now uses a default or explicit timeout.
### Deleted

## v0.9.3 - 2021-05-03
### Changed
- Fix crate metadata.

## v0.9.2 - 2021-05-03
### Changed
- Dependencies updated.
- Lowered Context drop error log to trace.

## v0.9.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.9.0 - 2021-04-19
### Changed
- Fix return route while sending message.
- Dependencies updated.

## v0.8.0 - 2021-04-14
### Added
- Added an extra-tracing environment variable.
- Added dead_code lint.
- Added runtime getter to Node.
- Enabled multi-hop routes via domain specific routers.
- Gracefully handle an already initialised tracing system.

### Changed
- Use ockam_node `block_on` to avoid blocking tokio executor.

## v0.7.0 - 2021-04-13
### Changed
- Dependencies updated.
- Renamed Context address functions.

## v0.6.0 - 2021-04-12
### Added
- None to Node Error enum.
- Re-add block_future functionality.
- `send_message_from_address` added to Context.


### Changed
- Fixed panic when main Context goes out of scope.
- Refactored Node Context API.
- `msg_addr` moved from `Context` to `Routed`.
- Node won't spawn a worker with a with colliding address.

### Deleted
- Excess clones.

## v0.5.0 - 2021-04-05
### Added
- Expose onward route information to user workers.
- Make context receive retry receiving if wrong type.
- Handle message payloads without inner length.
- Make consuming cancel wrapper yield routed wrapper
- Use special message parsing for context receive.

### Changed
- Dependencies updated.
- Log worker errors instead of panicking.

### Deleted

## v0.4.0 - 2021-03-22
### Added

- Routing APIs.
- Router registration.
- Message forwarding.
- Mechanism for stopping a single worker.
- The `receive_match` API allows workers to block until receivng a specific message type.


### Changed

- Dependency updates.
- External router implementations don't need to accept TransportMessage, or parse specific user message types.
- Improved logging.

## v0.3.0 - 2021-03-04
### Added

- Worker mailbox queues.
- Support new Context API in `ockam_node_attribute`
- Message cancellation.
- Receive returns a result.

### Changed
- Refactor of `node` API.
- Node, Context and Worker APIs are now async.
- Worker initialization is now async.
- Dependency updates.
- Context `send_message` no longer silently eats errors.

## v0.2.0 - 2021-02-16

### Added
- Message trait implementation.
- Messages for starting and stopping Workers.

### Changed
- Changed internal registry implementation.

### Deleted
- Previous Message implementation (NodeMessage)
- Relay abstraction prototype

## v0.1.1 - 2021-02-04
### Added

- Added description to Cargo.toml.

## v0.1.0 - 2021-02-03
### Added

- This document and other meta-information documents.
