# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

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
