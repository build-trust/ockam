# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.59.0 - 2022-05-23

### Changed

- Return socket address when starting a transport listener
- Updated dependencies

### Fixed

- Enable `SpanTrace` capture during tracing registration

## 0.58.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.57.0 - 2022-05-05

### Changed

- Updated dependencies

## 0.16.0 - 2022-05-04

### Changed

- Updated dependencies

## 0.15.0 - 2022-05-04

### Changed

- Adjust session timeout
- Updated dependencies

## 0.14.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.13.0 - 2022-04-19

### Changed

- Updated dependencies

### Fixed

- Fix ockam_command errors

## 0.12.0 - 2022-04-11

### Added

- Add session management
- Add session management to cli

### Changed

- Vault updates
- Make `Identity` trait immutable
- Updated dependencies

### Fixed

- Ensure that the command supports `OCKAM_LOG`
- Fix session ids handling in `ockam_command`

## 0.11.0 - 2022-04-04

### Changed

- Updated dependencies

## 0.10.0 - 2022-03-28

### Changed

- Updated dependencies

## 0.5.0 - 2022-01-26

### Changed

- Commands for inlet and outlet
- Ssh secure channel echoer cli

### Fixed

- Fix error handling in channel, cargo update

## 0.4.0 - 2022-01-10

### Added

- Add no_main arg support to ockam::node macro

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.3.0 - 2021-12-13

### Changed

- Updated deps

## 0.2.0 - 2021-12-06

### Fixed

- Rename ockam binary to ockam-cli to fix #2292

