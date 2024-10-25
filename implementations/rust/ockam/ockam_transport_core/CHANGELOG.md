# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.96.0 - 2024-10-25

### Added

- Updated dependencies

## 0.95.0 - 2024-10-24

### Added

- Check capabilities before using ebpf portals
- Updated dependencies

## 0.94.0 - 2024-10-23

### Added

- Updated dependencies

### Fixed

- Hostname validator now allows underscores in the hostname

## 0.93.0 - 2024-10-16

### Added

- `eBPF` portal updates:
- Updated dependencies

## 0.92.0 - 2024-10-11

### Added

- Updated dependencies

## 0.91.0 - 2024-09-23

### Added

- Improve ux of influxdb portal commands
- Set url dep as optional on ockam_transport_core
- Updated dependencies

## 0.90.0 - 2024-08-14

### Added

- Updated dependencies

## 0.89.0 - 2024-08-12

### Added

- Updated dependencies

## 0.88.0 - 2024-08-06

### Added

- Updated dependencies

## 0.87.0 - 2024-07-29

### Added

- Implicitly resolve outlet addresses during connection
- Converted socket addresses to hostnames in command
- Remove sync operations
- Updated dependencies

## 0.86.0 - 2024-07-03

### Added

- Display a malformed transport address
- Updated dependencies

## 0.85.0 - 2024-07-01

### Added

- Improve transport imports
- Change tcp protocol serialization
- Optimize cbor encoding by preallocating memory
- Updated dependencies

### Fixed

- Account for `minicbor` length calculation bug

## 0.84.0 - 2024-06-25

### Added

- Updated dependencies

## 0.83.0 - 2024-06-11

### Added

- Updated dependencies

## 0.82.0 - 2024-05-24

### Added

- Updated dependencies

## 0.81.0 - 2024-04-30

### Added

- Updated dependencies

## 0.80.0 - 2024-04-23

### Added

- Updated dependencies

## 0.79.0 - 2024-04-08

### Added

- Updated dependencies

## 0.78.0 - 2024-04-01

### Added

- Updated dependencies

### Fixed

- Decode a transport message even without a tracing_context field

## 0.77.0 - 2024-03-25

### Added

- Updated dependencies

## 0.76.0 - 2024-03-11

### Added

- Updated dependencies

## 0.75.0 - 2024-02-21

### Added

- Updated dependencies

### Changed

- Separate transport messages from local messages

## 0.74.0 - 2024-01-09

### Added

- Use `From` for converting errors
- Improve `TCP`:
- Updated dependencies

## 0.73.0 - 2024-01-04

### Added

- Added check for max message length
- Updated dependencies

## 0.72.0 - 2023-12-26

### Changed

- Updated dependencies

## 0.71.0 - 2023-12-16

### Changed

- Updated dependencies

## 0.70.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.69.0 - 2023-12-11

### Changed

- Updated dependencies

## 0.68.0 - 2023-12-06

### Changed

- Updated dependencies

## 0.67.0 - 2023-12-05

### Changed

- Updated dependencies

## 0.66.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.65.0 - 2023-11-08

### Changed

- Updated dependencies

## 0.64.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.63.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.62.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.61.0 - 2023-10-07

### Changed

- Updated dependencies

## 0.60.0 - 2023-10-05

### Changed

- Updated dependencies

## 0.59.0 - 2023-09-28

### Changed

- Updated dependencies

### Fixed

- Tungstenite 0.20.0 -> 0.20.1 bump related changes

## 0.58.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.57.0 - 2023-09-06

### Changed

- Updated dependencies

## 0.56.0 - 2023-06-26

### Changed

- Updated dependencies

## 0.55.0 - 2023-06-09

### Changed

- Updated dependencies

## 0.54.0 - 2023-05-26

### Changed

- Move `FlowControls` to `Context` and make it mandatory
- Updated dependencies

## 0.53.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Automate the creation and update of readmes
- Updated dependencies

## 0.52.0 - 2023-04-27

### Changed

- Move the route resolution to the context
- Updated dependencies

### Fixed

- Resolve transport addresses as a separate step

## 0.51.0 - 2023-04-14

### Changed

- Rename `Sessions` -> `FlowControls`
- Updated dependencies

## 0.50.0 - 2023-03-28

### Changed

- Clean `TrustOptions` processing
- Updated dependencies

## 0.49.0 - 2023-03-03

### Changed

- Updated dependencies

## 0.48.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.47.0 - 2023-02-09

### Changed

- Updated dependencies

## 0.46.0 - 2023-01-31

### Changed

- Updated dependencies

## 0.44.0 - 2022-11-08

### Changed

- Updated dependencies

## 0.43.0 - 2022-09-21

### Changed

- Updated dependencies

## 0.42.0 - 2022-09-09

### Changed

- Updated dependencies

## 0.41.0 - 2022-09-07

### Changed

- Updated dependencies

## 0.40.0 - 2022-09-05

### Changed

- Updated dependencies

## 0.39.0 - 2022-08-31

### Changed

- Updated dependencies

## 0.38.0 - 2022-08-29

### Changed

- Updated dependencies

## 0.37.0 - 2022-08-17

### Changed

- Updated dependencies

## 0.36.0 - 2022-08-12

### Changed

- Updated dependencies

## 0.35.0 - 2022-08-04

### Changed

- Updated dependencies

## 0.28.0 - 2022-06-06

### Changed

- Updated dependencies

## 0.27.0 - 2022-05-09

### Changed

- Updated dependencies

## 0.26.0 - 2022-04-25

### Changed

- Updated dependencies

## 0.25.0 - 2022-04-19

### Changed

- Build error mapping for various crates
- Clean up ockam_core import paths
- Update broken tests
- Rename error2 to error
- Updated dependencies

### Fixed

- Fixing lints
- Fix various clippy and rustfmt lints

### Removed

- Remove thiserror as it does not support no_std

## 0.24.0 - 2022-04-11

### Changed

- Implement tcp disconnection
- Implement miniature `ockam` command for demo
- Updated dependencies

### Fixed

- Insert a temporary mechanism to improve error messages

## 0.23.0 - 2022-03-28

### Changed

- Move `error_test.rs` into `error.rs` for project consistency
- Updated dependencies

### Fixed

- Fix non-exhaustive error code test in `code_and_domain()`

## 0.20.0 - 2022-02-08

### Changed

- Update crate edition to 2021

## 0.17.0 - 2022-01-10

### Changed

- Improve formatting of `Cargo.toml`s  and add `rust-version` 1.56.0

## 0.16.0 - 2021-12-13

### Changed

- Upgrade portals flow

## 0.15.0 - 2021-12-06

### Changed

- Make transport errors start from 1

### Removed

- Remove symlinks to `DEVELOP.md` and `LICENSE`

## v0.14.0 - 2021-11-22
- Dependencies updated


## v0.13.0 - 2021-11-15
### Changed
- Dependencies updated

## v0.12.0 - 2021-11-08
### Changed
- Dependencies updated

## v0.11.0 - 2021-11-01
### Changed
- Dependencies updated

## v0.10.0 - 2021-10-25
### Changed
- Move as many things as possible into a workspace.
- Dependencies updated

## v0.9.0 - 2021-10-18
### Added
- Added new 'no_main' feature to control ockam_node_attribute behavior on bare metal platforms
- Support no_std+alloc builds for ockam_transport_core
### Changed
- Dependencies updated

## v0.8.0 - 2021-10-11
### Changed
- Dependencies updated

## v0.7.0 - 2021-10-04
### Changed
- Dependencies updated

## v0.6.0 - 2021-09-27
### Changed
- Dependencies updated

## v0.5.0 - 2021-09-20
### Changed
- Dependencies updated

## v0.4.0 - 2021-09-14
### Changed
- Fixed incorrect link in README

## v0.3.0 - 2021-09-13
### Changed
- Dependencies updated.

## v0.2.0 - 2021-09-03
### Changed
- Dependencies updated.

## v0.1.0 - 2021-08-30
### Initial release
- Created ockam_transport_core crate for generic transport code
- `TransportError` - an enum with the common transport errors
