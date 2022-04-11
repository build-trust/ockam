# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

