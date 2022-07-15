# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.65.0 - 2022-07-15

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes

### Removed

- Remove custom validator on authenticated command

## 0.64.0 - 2022-07-15

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes

### Removed

- Remove custom validator on authenticated command

## 0.63.0 - 2022-06-30

### Added

- Add secure channel support to ockam_command
- Add command to create forwarders

### Changed

- Strategy to enable/disable logs in ockam_command
- Move `multiaddr_to_route` to `ockam_api`
- Change transport create command to addon command
- Make ockam command configuration thread safe

## 0.62.0 - 2022-06-17

### Changed

- Flatten overwrite field

## 0.61.0 - 2022-06-14

### Added

- Add commands to create and authenticate tokens
- Add configuration management to ockam_command
- Add email enrollment flow

### Changed

- Move nodeman protocol definitions to submodule
- Implement transport creation via ockam command
- Minicbor typetags, cli-cloud advances

### Fixed

- Improve the usability of ockam command

## 0.60.0 - 2022-06-06

### Added

- Add command-line interface for nodes api
- Add node subcommand
- Add message subcommand
- Add cloud enroll, space and project subcommands
- Add auth api to ockam_command
- Add clould invitation subcommands
- Add enrollment token + fixes to other commands

### Changed

- Use multi-address in ockam command
- Move old commands to a submodule
- Hide old subcommands from command help
- Rename dry_run command argument to test_argument_parser
- Enroll, project and space commands
- Improve ockam command help
- Improve ockam node command help
- Define command help template
- Turn cloud commands into top level commands
- Combine node start and spawn commands as create
- Allow ockam_command to call its own binary path
- Implement basic ockam_command config module
- Integrate configuration and remote messaging
- Basic node lifecycle management in ockam_command
- Utility function to purge all nodes
- Rename auth sub command to authenticated
- Run the authenticated service on node create
- Avoid `ockam_identity` dependency in `ockam_api`
- Updated dependencies

### Fixed

- Spawn node with ockam node create
- Log when tracing logging failed to initialise
- Hide tracing logging on client-side ockam cli instance

### Removed

- Remove ockam command spawn marker option
- Remove reqwest dependency in ockam_api

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

