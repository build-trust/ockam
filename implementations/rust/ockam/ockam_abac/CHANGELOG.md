# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.15.0 - 2023-02-24

### Changed

- Pre-trusted identity identifiers attributes
- Override inlet policy when starting kafka services
- Renamed abac implicit attribute subject.identity to identifier
- Updated dependencies

## 0.14.0 - 2023-02-09

### Changed

- Updated dependencies

### Fixed

- Apply `clippy --fix`

## 0.13.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.11.0 - 2022-11-08

### Added

- Add small language for abac
- Add `PolicyAccessControl`
- Add policy command
- Add command to list policies of a resource
- Add null expression

### Changed

- Implement `PolicyStorage` trait for lmdb
- Codespell implementations/rust/
- Avoid recursion when processing policies
- Complete policy delete functionality
- Always load policy from storage
- Evaluate null == null to false
- Updated dependencies

### Fixed

- Manually implement `PartialEq` and `PartialOrd`
- Align features with other crates

## 0.10.0 - 2022-09-21

### Changed

- Updated dependencies

## 0.9.0 - 2022-09-09

### Changed

- Updated dependencies

## 0.8.0 - 2022-09-07

### Changed

- Updated dependencies

## 0.7.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.6.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.5.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.4.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.3.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.2.0 - 2022-08-04

### Changed

- Updated dependencies

## v0.1.0 - 2022-07-20

- Initial release.
