# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.72.0 - 2024-10-25

### Added

- Updated dependencies

## 0.71.0 - 2024-10-24

### Added

- Updated dependencies

## 0.70.0 - 2024-10-23

### Added

- Updated dependencies

## 0.69.0 - 2024-10-16

### Added

- Updated dependencies

## 0.68.0 - 2024-10-11

### Added

- Updated dependencies

## 0.67.0 - 2024-09-23

### Added

- Implement influxdb token lessor service
- Updated dependencies

## 0.66.0 - 2024-08-14

### Added

- Updated dependencies

## 0.65.0 - 2024-08-12

### Added

- Updated dependencies

## 0.64.0 - 2024-08-06

### Added

- Updated dependencies

## 0.63.0 - 2024-07-29

### Added

- Updated dependencies

## 0.62.0 - 2024-07-03

### Added

- Updated dependencies

### Changed

- Use a published dependency for the patched sqlx library

## 0.61.0 - 2024-07-01

### Added

- Use the any driver for sqlx to add support for postgres
- Optimize cbor encoding by preallocating memory
- Updated dependencies

## 0.60.0 - 2024-06-25

### Added

- Exposed and added `ockam-rely` attribute validation for relay service
- Updated dependencies

### Changed

- Bump wast from 207.0.0 to 211.0.1

## 0.59.0 - 2024-06-11

### Added

- Updated dependencies

## 0.58.0 - 2024-05-30

### Added

- Updated dependencies

## 0.57.0 - 2024-05-25

### Added

- Create a parser for boolean expressions
- Add the possibility to use boolean expressions for policy expressions
- Address review comments
- Added abac rules to kafka inlet and oulet
- Introduce granular ac for kafka portal worker
- Allow kafka portals to anchor trust on identities
- Updated dependencies

## 0.56.0 - 2024-04-30

### Added

- Updated dependencies

## 0.55.0 - 2024-04-23

### Added

- Scope some repositories to a given node name
- Updated dependencies

## 0.54.0 - 2024-04-12

### Added

- Use outgoing access control
- Updated dependencies

## 0.53.0 - 2024-04-01

### Added

- Updated dependencies

## 0.52.0 - 2024-03-25

### Added

- Updated dependencies

## 0.51.0 - 2024-03-18

### Added

- Updated dependencies

## 0.50.0 - 2024-02-28

### Added

- Delete `TrustContext`
- Instrument more functions for enrollement
- Add policy migration that removes `trust_context_id`
- Introduce `subject.has_credential`
- Add policies for resource types
- Updated dependencies

### Changed

- Move the handling of attributes expiration date to a layer above the repository
- Optimize debug implementation for `PolicyAccessControl`

### Fixed

- Fix identity attributes expiration
- Store policies isolated by node and resource
- Fix policy storage expression type
- Use the correct policies in inlets/outlets created by kafka services

### Removed

- Remove `--resource` and `--resource-type` args from `policy show|list|delete`

## 0.49.0 - 2024-01-09

### Added

- Updated dependencies

## 0.48.0 - 2024-01-04

### Added

- Updated dependencies

## 0.47.0 - 2023-12-26

### Changed

- Updated dependencies

### Fixed

- Fix action variable in `PolicyAccessControl` `Debug` impl

## 0.46.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.45.0 - 2023-12-16

### Changed

- Persist application data in a database
- Implement policies and add consumer in the node manager worker
- Updated dependencies

## 0.44.0 - 2023-12-15

### Changed

- Updated dependencies

## 0.43.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.42.0 - 2023-12-11

### Changed

- Implement policies and add consumer in the node manager worker
- Updated dependencies

## 0.41.0 - 2023-12-06

### Changed

- Persist application data in a database
- Updated dependencies

## 0.40.0 - 2023-12-05

### Changed

- Persist application data in a database
- Updated dependencies

## 0.39.0 - 2023-11-23

### Changed

- Updated dependencies

## 0.38.0 - 2023-11-17

### Changed

- Updated dependencies

## 0.37.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.36.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.35.0 - 2023-11-02

### Changed

- Updated dependencies

## 0.34.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.33.0 - 2023-10-25

### Changed

- Updated dependencies

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
