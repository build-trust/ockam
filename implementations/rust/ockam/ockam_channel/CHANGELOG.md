# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.42.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.39.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.38.0 - 2021-12-13

### Changed

- Update `LocalInfo` logic
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

## 0.37.0 - 2021-12-06

### Changed

- Merge macro crates

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.36.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development
- Run `cargo clippy --fix`


## v0.35.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.34.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.33.0 - 2021-11-01
### Changed
- Explicitly derive Message trait
- Dependencies updated

## v0.32.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.31.0 - 2021-10-25
### Changed
- Make SecureChannel APIs async.
- Make async-trait crate used through ockam_core.
- Replace instances of `&Vec<T>` with `&[T]`.
- Dependencies updated
### Removed
- Remove `None` errors from Error enums.

## v0.30.0 - 2021-10-18
### Changed
- Dependencies updated

## v0.29.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.28.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.27.0 - 2021-09-27
### Changed
- Dependencies updated
- Use forked version of crates core2 and serde_bare.
- Ockam compiles under no_std + alloc.

## v0.26.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.25.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

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

