# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.71.0 - 2023-02-24

### Changed

- Use credential instead of credentials
- Inlined the ockam_key_exchange_core crate into the ockam_core crate
- Updated dependencies

## 0.70.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.69.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.67.0 - 2022-11-08

### Changed

- Updates for clippy 0.1.62
- Switch to arch agnostic integers for secret length
- Updated dependencies

### Fixed

- Move async tests to using `ockam_macros::test` to prevent hanging on panic

## 0.66.0 - 2022-09-21

### Changed

- Updates for clippy 0.1.62
- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.65.0 - 2022-09-09

### Changed

- Updates for clippy 0.1.62
- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.64.0 - 2022-09-07

### Changed

- Updates for clippy 0.1.62
- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.63.0 - 2022-09-05

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.62.0 - 2022-08-31

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.61.0 - 2022-08-29

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.60.0 - 2022-08-17

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.59.0 - 2022-08-12

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.58.0 - 2022-08-04

### Changed

- Updates for clippy 0.1.62
- Updated dependencies

## 0.56.0 - 2022-07-18

### Changed

- Updates for clippy 0.1.62

## 0.55.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.54.0 - 2022-07-15

### Changed

- Updates for clippy 0.1.62

## 0.51.0 - 2022-06-14

### Changed

- Create node builder for easier node initialisation

## 0.50.0 - 2022-06-06

### Changed

- Switch `Vault` to `String` `KeyId` instead of integer `Secret`
- Updated dependencies

### Removed

- Remove `AsRef` from `PublicKey` to avoid confusion

## 0.49.0 - 2022-05-23

### Changed

- Updated dependencies

## 0.48.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.47.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.46.0 - 2022-04-25

### Added

- Add "crate" attribute to async_try_clone_derive macro

### Changed

- Updated dependencies

## 0.45.0 - 2022-04-19

### Changed

- Build error mapping for various crates
- Clean up ockam_core import paths
- Run rustfmt
- Rename error2 to error
- Updated dependencies

### Fixed

- Fix various clippy and rustfmt lints

## 0.44.0 - 2022-04-11

### Changed

- Implement miniature `ockam` command for demo
- Vault updates
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages

## 0.43.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.42.0 - 2022-03-28

### Changed

- Updated dependencies

## 0.39.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.36.0 - 2022-01-10

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.35.0 - 2021-12-13

### Changed

- Vault updates
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

## 0.34.0 - 2021-12-06

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.33.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development
- Run `cargo clippy --fix`


## v0.32.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.31.0 - 2021-11-08
### Changed
- Dependencies updated
- replace `AsyncTryClone` trait impls with `#[derive(AsyncTryClone)]` wherever applicable

## v0.30.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.29.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.28.0 - 2021-10-25
### Changed
- Fix zeroize usage.
- Make APIs async.
- Make key exchange async.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated
### Removed
- Move as many things as possible into a workspace.

## v0.27.0 - 2021-10-18
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated

## v0.26.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.25.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.24.0 - 2021-09-27
### Changed
- Dependencies updated

## v0.23.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.22.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.21.0 - 2021-09-13
### Added
- Dependencies updated.

## v0.20.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.19.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.18.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.17.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
### Changed
- Dependencies updated.

## v0.16.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.15.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.14.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.12.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.11.0 - 2021-07-12
### Changed
- Dependencies updated.

## v0.10.0 - 2021-07-06
### Added
- Type for `BLS` secrets.

### Changed
- Dependencies updated.

## v0.9.0 - 2021-06-30
### Changed
- Fix clippy warnings.
- Dependencies updated.

## v0.8.0 - 2021-06-21
### Added
- Added LocalMessage for locally routed messages.
### Changed
- Dependencies updated.

## v0.7.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.6.0 - 2021-05-30
### Added
### Changed
- Dependency updates.
- Fix clippy issues.
- Populate key exchange changes.

## v0.5.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
### Deleted

## v0.4.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
### Deleted

## v0.3.3 - 2021-05-03
### Changed
- Fix crate metadata.

## v0.3.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.3.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.3.0 - 2021-04-22
### Changed
- Vault struct renames.

## v0.2.0 - 2021-04-19
### Changed
- Update XX to use VaultWorker.
- Dependencies updated.

## v0.1.4 - 2021-04-14
### Changed
- Build system and test fixes.
- Dependencies updated.

## v0.1.3 - 2021-04-13
### Changed
- Dependencies updated.

## v0.1.2 - 2021-04-12
### Changed
- Dependencies updated.

## v0.1.1 - 2021-04-06
### Changed
- Dependencies updated.

## v0.1.0 - 2021-04-05

- Initial release.
