# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.69.0 - 2025-01-20

### Added

- Updated dependencies

## 0.68.0 - 2025-01-09

### Added

- Updated dependencies

## 0.67.0 - 2024-12-12

### Added

- Add `UDP` support to nodes and multiaddr. refactor multiaddr
- Updated dependencies

## 0.66.0 - 2024-12-04

### Added

- Updated dependencies

## 0.65.0 - 2024-11-27

### Added

- Updated dependencies

## 0.64.0 - 2024-11-12

### Added

- Updated dependencies

### Fixed

- Error chain is kept in ockam_command crate

## 0.63.0 - 2024-10-23

### Added

- Updated dependencies

## 0.62.0 - 2024-10-11

### Added

- Updated dependencies

## 0.61.0 - 2024-09-23

### Added

- Updated dependencies

## 0.60.0 - 2024-08-14

### Added

- Updated dependencies

## 0.59.0 - 2024-08-12

### Added

- Updated dependencies

## 0.58.0 - 2024-08-06

### Added

- Updated dependencies

## 0.57.0 - 2024-07-29

### Added

- Updated dependencies

## 0.56.0 - 2024-07-01

### Added

- Optimize cbor encoding by preallocating memory
- Updated dependencies

## 0.55.0 - 2024-06-25

### Added

- Updated dependencies

## 0.54.0 - 2024-06-11

### Added

- Updated dependencies

## 0.53.0 - 2024-05-24

### Added

- Allow to specify 'self' in multiaddr
- Updated dependencies

## 0.52.0 - 2024-04-30

### Added

- Updated dependencies

## 0.51.0 - 2024-04-23

### Added

- Updated dependencies

## 0.50.0 - 2024-04-12

### Added

- Updated dependencies

## 0.49.0 - 2024-04-01

### Added

- Updated dependencies

## 0.48.0 - 2024-03-25

### Added

- Updated dependencies

## 0.47.0 - 2024-03-11

### Added

- Updated dependencies

## 0.46.0 - 2024-02-26

### Added

- Updated dependencies

### Fixed

- Fix clippy warnings on nightly

## 0.45.0 - 2024-01-09

### Added

- Updated dependencies

## 0.44.0 - 2024-01-04

### Added

- Updated dependencies

## 0.43.0 - 2023-12-26

### Changed

- Updated dependencies

## 0.42.0 - 2023-12-16

### Changed

- Updated dependencies

## 0.41.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.40.0 - 2023-12-11

### Changed

- Updated dependencies

## 0.39.0 - 2023-12-06

### Changed

- Updated dependencies

## 0.38.0 - 2023-12-05

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

- Refactored a bit the relay creation
- Updated dependencies

### Fixed

- Fix clippy warnings

## 0.30.0 - 2023-10-05

### Changed

- Refactored a bit the relay creation
- Updated dependencies

### Fixed

- Fix clippy warnings

## 0.29.0 - 2023-09-28

### Changed

- Updated dependencies

## 0.28.0 - 2023-09-23

### Changed

- Updated dependencies

## 0.27.0 - 2023-09-22

### Changed

- Updated dependencies

## 0.26.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.25.0 - 2023-09-06

### Changed

- Updated dependencies

## 0.24.0 - 2023-06-26

### Changed

- Updated dependencies

## 0.23.0 - 2023-06-09

### Changed

- Updated dependencies

## 0.22.0 - 2023-05-26

### Changed

- Improve `ockam_transport_tcp` registry
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

- Updated dependencies

## 0.18.0 - 2023-04-14

### Changed

- Implement custom get_env
- Updated dependencies

## 0.17.0 - 2023-03-28

### Changed

- Move `multiaddr_to_socket_addr` method into `MultiAddr`
- Lint a few files
- Updated dependencies

## 0.16.0 - 2023-03-03

### Changed

- Parse `/node/n1` to `/worker/addr` after connecting to the node via tcp
- Updated dependencies

## 0.15.0 - 2023-02-24

### Changed

- Updated dependencies

## 0.14.0 - 2023-02-09

### Changed

- Updated dependencies

### Fixed

- Apply `clippy --fix`

## 0.13.0 - 2023-01-31

### Added

- Add influxdb lease commands, orchestrator client, and default project

### Changed

- Updated dependencies

## 0.11.0 - 2022-11-08

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr
- Add `MultiAddr::{try_extend, try_with}`
- Add `secure` protocol to multi-addr
- Add `MultiAddr::matches`

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Require a name for secure protocol
- Recover tcp inlet
- Updated dependencies

### Fixed

- Mutliaddr support for projects
- Review feedback

## 0.10.0 - 2022-09-21

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr
- Add `MultiAddr::{try_extend, try_with}`

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.9.0 - 2022-09-09

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr
- Add `MultiAddr::{try_extend, try_with}`

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.8.0 - 2022-09-07

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr
- Add `MultiAddr::{try_extend, try_with}`

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.7.0 - 2022-09-05

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr
- Add `MultiAddr::{try_extend, try_with}`

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.6.0 - 2022-08-31

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.5.0 - 2022-08-29

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr
- Add serde feature to multiaddr
- Add cbor support to multiaddr

### Changed

- Rename ockam to service in multiaddr
- Allow project metadata lookups and route substitution
- Updated dependencies

### Fixed

- Mutliaddr support for projects

## 0.4.0 - 2022-08-17

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr

### Changed

- Rename ockam to service in multiaddr
- Updated dependencies

## 0.3.0 - 2022-08-12

### Added

- Add node protocol to multiaddr
- Add ways to push `ProtocolValue`s to multi-addr

### Changed

- Rename ockam to service in multiaddr
- Updated dependencies

## 0.2.0 - 2022-06-06

### Added

- Add pop_front to multi-address
- Add ockam protocol
- Add (inefficient) push_front

### Changed

- Initial implementation of multiaddr
- Tweak protocols
- Use multi-address in ockam command
- Updated dependencies

