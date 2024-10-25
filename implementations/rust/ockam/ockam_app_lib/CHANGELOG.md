# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.140.0 - 2024-10-25

### Added

- Updated dependencies

## 0.139.0 - 2024-10-24

### Added

- Add more granular scopes for command logs
- Allow relay connection failure without failing relay creation
- Updated dependencies

## 0.138.0 - 2024-10-23

### Added

- Updated dependencies

## 0.137.0 - 2024-10-21

### Added

- Change behavior of how nodes' processes are stopped
- Improvements to commands outputs
- Updated dependencies

## 0.136.0 - 2024-10-16

### Added

- Updated dependencies

## 0.135.0 - 2024-10-11

### Added

- Compact enrollment ticket encoded format
- Updated dependencies

## 0.134.0 - 2024-09-23

### Added

- Added `TLS` inlet support
- Influxdb inlet/outlet that attach authorization token
- Add reliable `TCP` portals to `ockam_api`&`ockam_command`
- Improve ux of influxdb portal commands
- Updated dependencies

## 0.133.0 - 2024-08-14

### Added

- Updated dependencies

## 0.132.0 - 2024-08-12

### Added

- Updated dependencies

## 0.131.0 - 2024-08-06

### Added

- Rework `Session`s
- Updated dependencies

## 0.130.0 - 2024-07-29

### Added

- Wait for the project to be ready before creating an authority client
- Implicitly resolve outlet addresses during connection
- Converted socket addresses to hostnames in command
- Remove sync operations
- Updated dependencies

## 0.129.0 - 2024-07-03

### Added

- Updated dependencies

### Changed

- Use a published dependency for the patched sqlx library

## 0.128.0 - 2024-07-01

### Added

- Improve transport imports
- Integrate `UDP` puncture into `ockam_api`
- Use the any driver for sqlx to add support for postgres
- Optimize cbor encoding by preallocating memory
- Updated dependencies

## 0.127.0 - 2024-06-25

### Added

- Add `identity` arg to `tcp-inlet create` to customize secure channel identifier
- Unified relay creation logic for project and rust
- Updated dependencies

### Fixed

- Avoid confusing logging when debugging the app

## 0.126.0 - 2024-06-11

### Added

- Updated dependencies

## 0.125.0 - 2024-05-30

### Added

- Create a portal for exporting traces when a project exists
- Updated dependencies

## 0.124.0 - 2024-05-24

### Added

- Improve output of `node show` and `status` commands
- Add the possibility to use boolean expressions for policy expressions
- Address review comments
- Add an http server to the node manager to return the node resources
- Updated dependencies

## 0.123.0 - 2024-04-30

### Added

- Updated dependencies

## 0.122.0 - 2024-04-23

### Added

- Support https for outlets
- Scope some repositories to a given node name
- Updated dependencies

## 0.121.0 - 2024-04-12

### Added

- Use outgoing access control
- Updated dependencies

## 0.120.0 - 2024-04-01

### Added

- Authority project admin credentials
- Create 3 separate credential retriever types
- Updated dependencies

## 0.119.0 - 2024-03-25

### Added

- Authority project admin credentials
- Updated dependencies

## 0.118.0 - 2024-03-18

### Added

- Add the node name to spans
- Updated dependencies

### Changed

- Rename methods and variables to insist on the exporting

## 0.117.0 - 2024-02-28

### Added

- Add support for additional kafka addons
- Add opentelemetry tracing and logging support
- Delete `TrustContext`
- Improve credentials management
- Backup logs when app restarts inlet node
- Instrument more functions for enrollement
- Unify creation and retry connection for portal and relay
- Pass the tracing context at the ockam message level
- Add policies for resource types
- Add an environment variable to configure a crates filter for log messages
- Refactor `Project`-related code
- Updated dependencies

### Changed

- Move the handling of attributes expiration date to a layer above the repository
- Enable tracing by default

### Fixed

- Command's verbose argument now has preference over env vars
- Fix identity attributes expiration
- Store policies isolated by node and resource

## 0.116.0 - 2024-01-09

### Added

- Use `From` for converting errors
- Updated dependencies

## 0.115.0 - 2024-01-04

### Added

- Updated dependencies

## 0.114.0 - 2023-12-26

### Changed

- Close unneeded tcp connections in various clients
- Updated dependencies

## 0.113.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.112.0 - 2023-12-18

### Changed

- Updated dependencies

## 0.15.0 - 2023-12-16

### Added

- Support self-invitation without breaking project enrollment

### Changed

- Persist application data in a database
- Split `node create` command code into separate files for background/foreground modes
- Updated dependencies

### Fixed

- Replace rolling appender to fix memory leak

## 0.14.0 - 2023-12-15

### Changed

- Split `node create` command code into separate files for background/foreground modes
- Updated dependencies

## 0.13.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.12.0 - 2023-12-11

### Added

- Support self-invitation without breaking project enrollment

### Changed

- Updated dependencies

### Fixed

- Replace rolling appender to fix memory leak

## 0.11.0 - 2023-12-06

### Changed

- Persist application data in a database
- Updated dependencies

## 0.10.0 - 2023-12-05

### Changed

- Persist application data in a database
- Updated dependencies

## 0.9.0 - 2023-11-23

### Added

- Added persistent state for incoming services in the app

### Changed

- Updated dependencies

## 0.8.0 - 2023-11-17

### Added

- Added persistent state for incoming services in the app

### Changed

- Updated dependencies

## 0.7.0 - 2023-11-08

### Changed

- Use `BackgroundNode` to handle tcp-inlets within the app
- Use `AuthorityNode` directly in the app to generate enrollment tickets
- Use `AuthorityNode` directly in the app to enroll to a project
- Updated dependencies

### Removed

- Remove nodes from the app without using the `CLI`

## 0.6.0 - 2023-11-08

### Changed

- Use `BackgroundNode` to handle tcp-inlets within the app
- Use `AuthorityNode` directly in the app to generate enrollment tickets
- Use `AuthorityNode` directly in the app to enroll to a project
- Updated dependencies

### Removed

- Remove nodes from the app without using the `CLI`

## 0.5.0 - 2023-11-02

### Changed

- Setup app's logs with the same features we use in the cli
- Updated dependencies

### Fixed

- Fixed app crashes during refresh and during reset/shutdown

## 0.4.0 - 2023-10-26

### Changed

- Polishing desktop app paper cuts
- Updated dependencies

### Fixed

- Fixes many issues and papercuts in swift app

## 0.3.0 - 2023-10-25

### Changed

- Updated dependencies

### Fixed

- Fixes many issues and papercuts in swift app

## 0.2.0 - 2023-10-18

### Added

- Added native ockam desktop app for macos

### Changed

- Updated dependencies

# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
