# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.71.0 - 2023-02-24

### Added

- Add more information to `EntryNotFound` errors

### Changed

- Bump aws-sdk-kms to 0.24.0 and aws-config to 0.54.1
- Updated dependencies

## 0.70.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.69.0 - 2023-01-31

### Changed

- Bump p256 from 0.11.1 to 0.12.0
- Updated dependencies

## 0.67.0 - 2022-11-08

### Changed

- Switch to arch agnostic integers for secret length
- Codespell implementations/rust/
- Complete hkdf version update
- Updated dependencies

### Removed

- Remove bls support

## 0.66.0 - 2022-09-21

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.65.0 - 2022-09-09

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.64.0 - 2022-09-07

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.63.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.62.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.61.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.60.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.59.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.58.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.51.0 - 2022-06-14

### Changed

- Move ockam_vault service to ockam_api

## 0.50.0 - 2022-06-06

### Added

- Add simple `Vault` service
- Add simple vault service test

### Changed

- Switch `Vault` to `String` `KeyId` instead of integer `Secret`
- Implement new `Vault` serialization
- Improve file handling in `Vault` storage
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

### Changed

- Updated dependencies

## 0.45.0 - 2022-04-19

### Changed

- Clean up ockam_core import paths
- Run rustfmt
- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam_vault
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.44.0 - 2022-04-11

### Changed

- Don't re-export `hex` or `hashbrown` from `ockam_core`
- Implement miniature `ockam` command for demo
- Vault updates
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages

## 0.43.0 - 2022-03-28

### Changed

- Friendlify api for `ockam_core::vault::key_id_vault`
- Updated dependencies

## 0.42.0 - 2022-03-21

### Fixed

- Vault_test macro
- Vault_test_sync macro

## 0.40.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.37.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

### Fixed

- Fix credentials build failure

## 0.36.0 - 2021-12-13

### Added

- Add ockam_core/bls feature and small fixes

### Changed

- Vault updates
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

## 0.35.0 - 2021-12-06

### Changed

- Merge macro crates

### Fixed

- Import correct ed25519 signature trait

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.34.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development

### Fixed

- Allow deprecated use of `Signature::new`
- Switch from `Signature::new` to `Signature::from_bytes`
- Prefix xeddsa sign and verify methods with `xeddsa_`


## v0.33.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.32.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.31.0 - 2021-11-01
### Changed
- explicitly derive message trait
- Dependencies updated

## v0.30.0 - 2021-10-25
### Changed
- Fix zeroize usage.
- Make vault async.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated
### Remove
- Remove `None` errors from Error enums.

## v0.29.0 - 2021-10-18
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated

## v0.28.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.27.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.25.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.24.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.23.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.22.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.21.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.20.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.19.0 - 2021-08-16
### Added
- Add key validation during secret_import in ockam_vault.
- Implement BLS signature using BBS+.
- Introduce Signature vault type.
### Changed
- Dependencies updated.
- Remove clamping from curve25519 key generation.

## v0.18.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.17.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.16.0 - 2021-07-29
### Added
- Add key id computation while importing secret in ockam_vault.
### Changed
- Dependencies updated.

## v0.15.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.14.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-12
### Added
- Placeholder for BLS signature Vault implementation.
### Changed
- Dependencies updated.

## v0.12.0 - 2021-07-06
### Added
- Type for `BLS` secrets.

### Changed
- Dependencies updated.

## v0.11.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Fix clippy warnings.
- Dependencies updated.

## v0.10.0 - 2021-06-21
### Changed
- Dependencies updated.

## v0.9.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.8.0 - 2021-05-30
### Added
### Changed
- Dependencies updated.

## v0.7.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
- Use result_message in Vault Worker.
- Refactors in support of Entity abstraction.

## v0.6.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
- Documentation edits.
### Deleted

## v0.5.2 - 2021-05-03
### Changed
- Dependencies updated.

## v0.5.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.5.0 - 2021-04-33
### Changed
- Crate dependency reorganization.

## v0.4.1 - 2021-04-19
### Changed
- Dependencies updated.

## v0.4.0 - 2021-04-14
### Changed
- Improved asymmetric trait.
- Improved hasher trait.
- Improved key_id trait.
- Improved verify trait.
- Build system and test fixes.

## v0.3.4 - 2021-04-13
### Changed
- Dependencies updated.

## v0.3.3 - 2021-04-12
### Changed
- Dependencies updated.

## v0.3.2 - 2021-04-06
### Changed
- Dependencies updated.

## v0.3.1 - 2021-03-23
### Changed
- Dependencies updated.

## v0.3.0 - 2021-03-03
### Changed

- Traits renamed for consistency.
- Documentation edits.

## v0.2.0 - 2021-02-16
### Added
- Symmetric and Asymmetric SoftwareVault implementations.

### Changed
- Dependencies updated.
- Fixes to error propagation and entity construction.

## v0.1.0 - 2021-02-10

Initial release.

