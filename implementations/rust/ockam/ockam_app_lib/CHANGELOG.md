# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
