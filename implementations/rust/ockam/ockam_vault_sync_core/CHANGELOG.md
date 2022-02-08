# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.38.0 - 2022-02-08

### Changed

- Async compat updates to identity and vault
- Update crate edition to 2021

## 0.35.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.34.0 - 2021-12-13

### Changed

- Change uses of `ockam_vault_core::Foo` to use `ockam_core::vault::Foo` across crates

## 0.33.0 - 2021-12-06

### Changed

- Merge macro crates

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`
- Remove need for separate macro crates

## v0.32.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development


## v0.31.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.30.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.29.0 - 2021-11-01
### Changed
- explicitly derive message trait
- Dependencies updated

## v0.28.0 - 2021-10-26
### Changed
- Clippy improvements
- Put `VaultMutex` behind `cfg(feature = "std")`
- Fix tokio dependency features
- Dependencies updated

## v0.27.0 - 2021-10-25
### Changed
- Fix zeroize usage.
- Make APIs async.
- Make async-trait crate used through ockam_core.
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated
### Removed
- Remove `None` errors from Error enums.

## v0.26.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
- Use ockam_core::compat::mutex instead of cortex_m::interrupt::*
### Changed
- Dependencies updated
- Make vault_sync use `Handle`

## v0.25.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.24.0 - 2021-10-04
### Added
- Implement AsyncTryClone for VaultSync
### Changed
- Dependencies updated

## v0.23.0 - 2021-09-27
### Changed
- Ockam compiles under no_std + alloc.
- Dependencies updated

## v0.22.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.21.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.20.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.19.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.18.0 - 2021-08-30
### Changed
- Dependencies updated.

## v0.17.0 - 2021-08-23
### Changed
- Replace std:: modules with core:: and alternate implementations
- Dependencies updated.

## v0.16.0 - 2021-08-16
### Added
- Implement BLS signature using BBS+.
- Introduce Signature vault type.
### Changed
- Dependencies updated.

## v0.15.0 - 2021-08-09
### Changed
- Dependencies updated.

## v0.14.0 - 2021-08-03
### Changed
- Dependencies updated.

## v0.13.0 - 2021-07-29
### Changed
- Dependencies updated.

## v0.12.0 - 2021-07-26
### Changed
- Dependencies updated.

## v0.11.0 - 2021-07-19
### Changed
- Dependencies updated.

## v0.10.0 - 2021-07-12
### Changed
- Dependencies updated.

## v0.9.0 - 2021-07-06
### Added
- Type for `BLS` secrets.

### Changed
- Dependencies updated.

## v0.8.0 - 2021-06-30
### Added
- Identity trait for defining Profile behavior.
### Changed
- Entity and Profile implementation restructured.
- Dependencies updated.

## v0.7.0 - 2021-06-21
### Changed
- Dependencies updated.

## v0.6.0 - 2021-06-14
### Changed
- Dependencies updated.

## v0.5.0 - 2021-05-30
### Added
### Changed
- Dependency updates.
- Fix clippy issues.

## v0.4.0 - 2021-05-17
### Added
### Changed
- Dependencies updated.
- Use result_message in Vault Worker.
- Refactors in support of Entity abstraction.

## v0.3.0 - 2021-05-10
### Added
### Changed
- Dependencies updated.
- Documentation edits.
### Deleted

## v0.2.2 - 2021-05-03
### Changed
- Dependencies updated.
- Vault creation is now sync.

## v0.2.1 - 2021-04-26
### Changed
- Dependencies updated.

## v0.2.0 - 2021-04-22
### Changed
- Vault struct renames.

## v0.1.0 - 2021-04-19
- Initial release.
