# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.16.0 - 2022-06-14

### Added

- Add `#[ockam::node]` macro attribute `access_control`

### Changed

- Create node builder for easier node initialisation

## 0.15.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.14.0 - 2022-04-25

### Added

- Add "crate" attribute to async_try_clone_derive macro

### Changed

- Cleanup cargo.toml and readme.md
- Updated dependencies

## 0.13.0 - 2022-04-19

### Changed

- Run rustfmt
- Updated dependencies

### Fixed

- Errors: fix ockam_transport_tcp

## 0.12.0 - 2022-04-11

### Added

- Add "crate" attribute to "node" macro

### Changed

- Vault updates
- Updated dependencies

## 0.11.0 - 2022-03-28

### Added

- Add more docs to the ockam macros

### Changed

- Redesign `node` and `test` macros
- `async_try_clone_derive` to follow new macro structure
- Updated dependencies

### Fixed

- Fix ockam::test macro tests
- Fix ockam::node macro tests
- Imports ockam context as used in the input function

### Removed

- Remove `todo`

## 0.10.0 - 2022-03-21

### Changed

- Improve error handling and make "entry" functions "pub(crate)"
- Rust docs for public functions in `ockam_macros`

### Fixed

- Message_derive macro
- Vault_test_sync macro
- Clean up node_test tests

### Removed

- Remove syn's `extra-traits` feature in `ockam_macro`

## 0.9.0 - 2022-03-07

### Added

- Add "crate" attribute to ockam_macros::test macro

## 0.8.0 - 2022-02-22

### Fixed

- Fix test macro timeout handling

## 0.7.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.4.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

### Fixed

- Fix no_std compilation error due to `ockam::node` attribute

## 0.3.0 - 2021-12-13

### Changed

- Updated dependencies

## 0.2.0 - 2021-12-06

### Changed

- Merge macro crates

### Removed

- Remove need for separate macro crates

## v0.1.0 - 2021-11-08

Initial release.

