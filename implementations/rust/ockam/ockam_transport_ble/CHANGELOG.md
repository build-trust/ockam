# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.45.0 - 2023-06-26

### Changed

- Updated dependencies

## 0.44.0 - 2023-06-09

### Changed

- Updated dependencies

## 0.43.0 - 2023-05-26

### Changed

- Use identity identifiers for the creation of secure channels
- Updated dependencies

## 0.42.0 - 2023-05-12

### Changed

- Updated dependencies

## 0.41.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Updated dependencies

## 0.40.0 - 2023-04-27

### Changed

- Extract identity as an entity
- Updated dependencies

## 0.39.0 - 2023-04-14

### Changed

- Introduce `TrustOptions::insecure()` and `::insecure_test()`
- Rename `insecure_test` -> `new`
- Rename `TrustOptions` -> `Options`
- Updated dependencies

## 0.38.0 - 2023-03-28

### Changed

- Updated dependencies

### Removed

- Remove the need for _arc functions

## 0.37.0 - 2023-03-03

### Changed

- Updated dependencies

## 0.36.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.35.0 - 2023-02-09

### Changed

- Updated dependencies

### Fixed

- Apply `clippy --fix`

## 0.34.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.32.0 - 2022-11-08

### Changed

- Updated dependencies

## 0.31.0 - 2022-09-21

### Changed

- Updated dependencies

## 0.30.0 - 2022-09-09

### Changed

- Updated dependencies

## 0.29.0 - 2022-09-07

### Changed

- Updated dependencies

## 0.28.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.27.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.26.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.25.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.24.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.23.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.18.0 - 2022-06-30

### Changed

- Update docs and examples

## 0.16.0 - 2022-06-14

### Changed

- Create node builder for easier node initialisation

## 0.15.0 - 2022-06-06

### Changed

- Rename new_context to new_detached
- Updated dependencies

## 0.14.0 - 2022-05-23

### Changed

- Updated dependencies

## 0.13.0 - 2022-05-09

### Changed

- Rename github organization to build-trust
- Updated dependencies

## 0.12.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.11.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.10.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.9.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.8.0 - 2022-04-19

### Changed

- Update broken tests
- Rename error2 to error
- Updated dependencies

### Fixed

- Errors: fix ockam_transport_ble
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.7.0 - 2022-04-11

### Changed

- Get rid of common `RouterMessage` in favor of transport-specific structs (ble, ws)
- Tune up some of the documentation
- Rename `mod remote_forwarder` module to `mod remote`, fix examples
- Implement miniature `ockam` command for demo
- Make `Identity` trait immutable
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages
- Fix clippy warnings

## 0.6.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.5.0 - 2022-03-28

### Changed

- Updated dependencies

## 0.2.0 - 2022-02-22

### Changed

- Initial commit for ockam_transport_ble crate

### Fixed

- Fix `BleTransport` initialization race condition

