# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## unreleased

### Changed

- Update crate edition to 2021

## 0.12.0 - 2022-01-10

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0
- Use the tracing crate for logging on no_std

## 0.11.0 - 2021-12-13

### Changed

- Updated dependencies

## 0.10.0 - 2021-12-06

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.9.0 - 2021-11-22


### Changed

- Deny warnings in ci, not local development


## v0.8.0 - 2021-11-15
### Changed
- add warn log macro to no_std
- avoid deadlocking the ockam_executor
- Dependencies updated

## v0.7.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.6.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.5.0 - 2021-10-25
### Changed
- Simplified feature usage.
- Move as many things as possible into a workspace.
- Dependencies updated

## v0.4.0 - 2021-10-18
### Changed
- Various improvements to ockam_executor
- Use ockam_core::compat::mutex instead of cortex_m::interrupt::*
- Dependencies updated

## v0.3.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.2.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.1.0 - 2021-09-28
### Added

- This document.
