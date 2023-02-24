# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.67.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.66.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.65.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.63.0 - 2022-11-08

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

### Removed

- Remove bls support

## 0.62.0 - 2022-09-21

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.61.0 - 2022-09-09

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.60.0 - 2022-09-07

### Changed

- Switch to arch agnostic integers for secret length
- Updated dependencies

## 0.59.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.58.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.57.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.56.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.55.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.54.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.46.0 - 2022-06-06

### Changed

- Switch `Vault` to `String` `KeyId` instead of integer `Secret`
- Updated dependencies

### Fixed

- Fix ffi after `Vault` updates

### Removed

- Remove `AsRef` from `PublicKey` to avoid confusion

## 0.45.0 - 2022-05-23

### Changed

- Updated dependencies

## 0.44.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.43.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.42.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.41.0 - 2022-04-19

### Changed

- Clean up ockam_core import paths
- Run rustfmt
- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam_ffi
- Fixing lints
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.40.0 - 2022-04-11

### Changed

- Vault updates
- Updated dependencies

## 0.39.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.38.0 - 2022-03-28

### Changed

- Updated dependencies

## 0.35.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.32.1 - 2022-01-10

### Added

- Add script to rebuild macos native vault lib
- Add ockam_core/bls feature and small fixes

### Changed

- Vault updates
- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates
- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## v0.32.0 - 2021-12-06

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.31.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development


## v0.30.0 - 2021-11-15
### Changed
- change `Doesnt` to `DoesNot` for enum variants
- Dependencies updated

## v0.29.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.28.0 - 2021-11-01
### Changed
- catch panics in `extern "C"` functions
- avoid leaking memory on errors in the ffi
- clean up `ockam_ffi`'s handling of `bls` feature
- Dependencies updated

## v0.27.0 - 2021-10-26
### Changed
- Dependencies updated

## v0.26.0 - 2021-10-25
### Changed
- Update FFI to async.
- Move as many things as possible into a workspace.
- Dependencies updated

### Removed
- Remove `None` errors from Error enums.

## v0.25.0 - 2021-10-18
### Changed
- Make credentials optional (disabled by default)
- Dependencies updated

## v0.24.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.23.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.22.0 - 2021-09-27
### Changed
- Dependencies updated
- Ockam compiles under no_std + alloc.

## v0.21.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.20.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.19.0 - 2021-09-13
### Changed
- Change `size_t` to `unint3_t` in C header file.
- Revert signature `output_buffer` type to a pointer.
- Dependencies updated.

## v0.18.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.17.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.16.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.15.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
### Changed
- Dependencies updated.

## v0.14.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.13.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.12.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.11.0 - 2021-07-26
### Added
### Changed
- Include stddef.h in ockam_ffi/include/vault.h
- Dependencies updated.

## v0.10.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.9.0 - 2021-07-12
### Changed
- Dependencies updated.

## v0.8.0 - 2021-07-06
### Added
- Type for `BLS` secrets.
### Changed
- Dependencies updated.

## v0.7.0 - 2021-06-30
### Changed
- Dependencies updated.

## v0.6.0 - 2021-06-21
### Changed
- Dependencies updated.

## v0.5.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.4.0 - 2021-05-30
### Added
### Changed
- Dependency updates.

## v0.3.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
### Deleted

## v0.2.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
### Deleted

# v0.1.7 - 2021-05-03
### Changed
- Fix crate metadata.

# v0.1.7 - 2021-05-03
### Changed
- Fix crate metadata.

# v0.1.6 - 2021-05-03
### Changed
- Dependencies updated.

# v0.1.5 - 2021-04-26
### Changed
- Dependencies updated.

# v0.1.4 - 2021-04-19
### Changed
- Dependencies updated.

# v0.1.3 - 2021-04-14
### Changed
- Dependencies updated.

# v0.1.2 - 2021-04-13
### Changed
- Dependencies updated.

# v0.1.1 - 2021-04-12
### Changed
- Dependencies updated.

# v0.1.0 - 2021-04-05
### Changed
- Initial release.
