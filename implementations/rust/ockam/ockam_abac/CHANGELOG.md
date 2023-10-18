# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.32.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.31.0 - 2023-10-07

### Changed

- Updated dependencies

## 0.30.0 - 2023-10-05

### Changed

- Updated dependencies

## 0.29.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.28.0 - 2023-09-23

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.27.0 - 2023-09-22

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.26.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.25.0 - 2023-09-06

### Changed

- Bump wast from 60.0.0 to 62.0.1
- Updated dependencies

## 0.24.0 - 2023-06-26

### Changed

- Updated dependencies

## 0.23.0 - 2023-06-09

### Changed

- Full local kafka implementation which credential validation and flow control
- Updated dependencies

## 0.22.0 - 2023-05-26

### Changed

- Updated dependencies

## 0.21.0 - 2023-05-12

### Changed

- Updated dependencies

## 0.20.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Updated dependencies

## 0.19.0 - 2023-04-27

### Changed

- Extract identity as an entity
- Move the lmdb storage
- Updated dependencies

## 0.18.0 - 2023-04-14

### Changed

- Updated dependencies

## 0.17.0 - 2023-03-28

### Changed

- Updated dependencies

### Removed

- Removed type parameters exposing implementation details

## 0.16.0 - 2023-03-03

### Changed

- Reuse the abac control policy inside the policy access control
- Use abac in authority services implementation
- Updated dependencies

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
