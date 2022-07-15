# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.7.0 - 2022-07-15

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service

## 0.6.0 - 2022-06-30

### Changed

- `Storage` -> `AuthenticatedTable`
- Identity updates
- `AuthenticatedTable` -> `AuthenticatedStorage`
- Move `multiaddr_to_route` to `ockam_api`
- Allow conversion from route to multiaddr
- Partially convert ockam_command to use multiaddr

## 0.4.0 - 2022-06-14

### Added

- Add `to_vec()` for `RequestBuilder` and `ResponseBuilder`

### Changed

- Move ockam_vault service to ockam_api
- Move ockam_identity service to ockam_api
- Update nodemanager service to ockam_api structures
- Move node manager service to ockam_api crate
- Minicbor typetags, cli-cloud advances

### Fixed

- Apply style feedback

## 0.3.0 - 2022-06-06

### Added

- Add builders to ockam_api
- Add ockam_api_nodes
- Add command-line interface for nodes api
- Add cloud enroll, space and project subcommands
- Add cowbytes and cowstr
- Add `into_owned` for `CowStr` and `CowBytes`
- Add pid query to nodeman worker
- Add auth api
- Add clould invitation subcommands
- Add enrollment token + fixes to other commands

### Changed

- Ensure command-line args are not empty
- Rename new_context to new_detached
- Improve schema validation
- Avoid `ockam_identity` dependency in `ockam_api`
- Change `Defer` type for `CowStr` and `CowBytes`
- Make `Method` enum exhaustive
- Move `TypeTag` to `ockam_core`
- Extend `Request` and `Response` encode api
- Updated dependencies

### Fixed

- Rename subject to authenticated

### Removed

- Remove reqwest dependency in ockam_api

## 0.2.0 - 2022-05-23

### Added

- Add ockam_api

### Changed

- Updated dependencies

