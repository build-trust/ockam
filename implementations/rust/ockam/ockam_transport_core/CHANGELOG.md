# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.48.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.47.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.46.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.44.0 - 2022-11-08

### Changed

- Updated dependencies

## 0.43.0 - 2022-09-21

### Changed

- Updated dependencies

## 0.42.0 - 2022-09-09

### Changed

- Updated dependencies

## 0.41.0 - 2022-09-07

### Changed

- Updated dependencies

## 0.40.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.39.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.38.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.37.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.36.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.35.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.28.0 - 2022-06-06

### Changed

- Updated dependencies

## 0.27.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.26.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.25.0 - 2022-04-19

### Changed

- Build error mapping for various crates
- Clean up ockam_core import paths
- Update broken tests
- Rename error2 to error
- Updated dependencies

### Fixed

- Fixing lints
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.24.0 - 2022-04-11

### Changed

- Implement tcp disconnection
- Implement miniature `ockam` command for demo
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages

## 0.23.0 - 2022-03-28

### Changed

- Move `error_test.rs` into `error.rs` for project consistency
- Updated dependencies

### Fixed

- Fix non-exhaustive error code test in `code_and_domain()`

## 0.20.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.17.0 - 2022-01-10

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.16.0 - 2021-12-13

### Changed

- Upgrade portals flow

## 0.15.0 - 2021-12-06

### Changed

- Make transport errors start from 1

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.14.0 - 2021-11-22
- Dependencies updated


## v0.13.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.12.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.11.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.10.0 - 2021-10-25
### Changed
- Move as many things as possible into a workspace.
- Dependencies updated

## v0.9.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
- Support no_std+alloc builds for ockam_transport_core
### Changed
- Dependencies updated

## v0.8.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.7.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.6.0 - 2021-09-27
### Changed
- Dependencies updated

## v0.5.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.4.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.3.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.2.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.1.0 - 2021-08-30
### Initial release
- Created ockam_transport_core crate for generic transport code
- `TransportError` - an enum with the common transport errors
