# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

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
- Builder for Routable messags.
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

