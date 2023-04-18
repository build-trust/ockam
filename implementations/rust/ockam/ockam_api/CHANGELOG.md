# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.27.0 - 2023-04-14

### Added

- Add trust context struct and traits
- Add trust context config and insantiate node manager with trust options
- Add trust context option to node create, use trust context with credential option
- Add more bats tests for trust context
- Add `RpcProxyService`
- Add a limited version of the `ockam run` command
- Add config directly to trust context state

### Changed

- Implement custom get_env
- Use trust context within the creation of ockam_api secure channels
- Trust context fully dictates cred check on node man
- Introduce `TrustOptions::insecure()` and `::insecure_test()`
- Start using `session_id` for outgoing secure channels in `ockam_api` and `ockam_command`
- Make message flow `Sessions` work with replacement `Sessions`
- Reduce usage of `::insecure()`
- Rename `create_tcp_session` -> `multiaddr_to_route`
- Rename `insecure_test` -> `new`
- Rename `Sessions` -> `FlowControls`
- Rename `TrustOptions` -> `Options`
- Use cli state for trust context and default trust context
- Disable `FlowControl` for loopback tcp connections and listeners
- Updated dependencies

### Fixed

- Fix project deletion from state
- Fix `authenticated` command & `Sessions`
- Fixes after tough rebase
- Include trust-context path in ockam reset

### Removed

- Remove few unwraps

## 0.26.0 - 2023-03-28

### Added

- Add `create_tcp_session` to `ockam_command`
- Add missing serialize / deserialize instances

### Changed

- Create tcp_connection along with secure channels in the same function call
- Use sessions in ockam_api
- Make trust arguments mandatory
- `Sessions` update
- Create an authority node
- Start the authority node with the node create command
- Retrieve the identity authority before creating the authority node
- Show the authority node as up
- Retry the creation of the lmdb database in case of a failure
- Refactor tuple to api-transport struct
- Move `multiaddr_to_socket_addr` method into `MultiAddr`
- Don't try to delete files or directories which are already deleted
- Updated dependencies

### Fixed

- Fixed the compilation errors with the tag feature
- Fix clippy warnings on test code
- Node duplication error
- Node duplication error
- Use the same criteria for checking if a node exists
- Make the authority_node field optional
- Make `ockam reset` delete specific state files
- When deleting the default vault/identity/project the data and the link are deleted

### Removed

- Remove warnings
- Removed type parameters exposing implementation details
- Remove the need for _arc functions
- Remove the legacy storage migration code

## 0.25.0 - 2023-03-03

### Added

- Add print encodable output

### Changed

- Refactor `CliState` so it can be built using an explicit directory
- Update `ockam_api` and `ockam_command` according to `TCP` updates
- Parse `/node/n1` to `/worker/addr` after connecting to the node via tcp
- Extend `ockam_api` transport info
- Use abac in authority services implementation
- Expand credential commands
- Update secure-channel create to allow for a provided credential
- Updated dependencies

### Fixed

- Fixes broken tests for macos, let the os choose available ports
- Reorganize bats tests to run them in parallel
- 'ockam enroll' ovewrites current configuration instead of returning error
- Update cli_state test with credentials entry

## 0.24.0 - 2023-02-24

### Added

- Add default subcommand to node

### Changed

- Pre-trusted identity identifiers attributes
- Use credential instead of credentials
- Usable kafka sidecar implementation
- Standarize where authority stores membership information
- Implemented kafka message encryption and orchestrator integration
- Bump aws-sdk-kms to 0.24.0 and aws-config to 0.54.1
- Split cddl schema files & merge when cbor api validation is needed
- Updated dependencies

### Fixed

- Deleting a vault won't affect the default

### Removed

- Remove the lifetime annotation on `Credential` and `Attributes`

## 0.23.0 - 2023-02-09

### Added

- Add command to set the default vault
- Add command to set the default identity

### Changed

- Recipient returns an error instead of panicking
- Nodestate implement check whether a node is running
- Updated dependencies

### Fixed

- Apply `clippy --fix`
- Deleting an identity won't affect the default

## 0.22.0 - 2023-01-31

### Added

- Add kafka commands to request starting the producer/consumer services
- Add flag to reload enrollers from a file
- Add influxdb lease commands, orchestrator client, and default project

### Changed

- Create `SecureChannelRegistry`
- Move `storage` and `registry` to `Identity`
- Refactor `CliState` so the `authenticated_storage` is stored in the identities dir
- Implement vaults delete command
- Updated dependencies

### Fixed

- Vault deletion logic from `CliState`

## 0.20.0 - 2022-11-08

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`
- Add `MultiAddr::matches`
- Add policy command
- Add command to list policies of a resource
- Add support to `project enroll` to set attributes

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Use `DefaultAddress` consts for default services addresses
- Change echo worker to accept any message
- Recover remote forwarder
- Resolve forwarder project name in manager
- `ockam node show` to use dynamic data from node
- Recover tcp inlet
- Use `Arc<RwLock<NodeManager>>` in recovery
- Implement `PolicyStorage` trait for lmdb
- Okta identity provider
- Complete policy delete functionality
- Wrap stored policy expressions
- Rename inlet and outlet policy resources
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings
- Cleanup
- Changes due to review comments
- Review feedback

### Removed

- Remove ability to set arbitrary attributes

## 0.19.0 - 2022-09-21

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Use `DefaultAddress` consts for default services addresses
- Change echo worker to accept any message
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings

### Removed

- Remove ability to set arbitrary attributes

## 0.18.0 - 2022-09-09

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Use `DefaultAddress` consts for default services addresses
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings

### Removed

- Remove ability to set arbitrary attributes

## 0.17.0 - 2022-09-07

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings

### Removed

- Remove ability to set arbitrary attributes

## 0.16.0 - 2022-09-05

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings

### Removed

- Remove ability to set arbitrary attributes

## 0.15.0 - 2022-08-31

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Authority config keys must be strings

### Removed

- Remove ability to set arbitrary attributes

## 0.14.0 - 2022-08-29

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate
- Add `Inlet/Outlet` to `Registry`

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Check controller's identity id when creating secure channel
- Always start signer service
- Replace signer with verifier
- Allow project metadata lookups and route substitution
- Change `VerifyRequest::credential` to binary
- Make `IdentityChangeHistory` crate public, cleanup usage
- Move credentials to `ockam_identity`
- Improve credential verification
- Get rid of old `ockam_api` module
- Return project names from multiaddr clean function
- Move project readiness logic into ockam_api
- Updated dependencies

### Fixed

- Clippy lints
- Fix schema validation
- Mutliaddr support for projects
- Creation of static forwarder at local nodes

### Removed

- Remove ability to set arbitrary attributes

## 0.13.0 - 2022-08-17

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store
- Add `credential` module to `ockam` crate

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Make `IdentityIdentifier` encodable
- Move `CowStr` and `CowBytes` to `ockam_core`
- Move api structs to `ockam_core`
- Updated dependencies

### Fixed

- Clippy lints

### Removed

- Remove ability to set arbitrary attributes

## 0.12.0 - 2022-08-12

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling
- Add project node identity to project cbor schema
- Add util::response module
- Add signer and direct enroller support
- Support different enroller/member store

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename ockam to service in multiaddr
- Integrate uppercase and echoer workers to nodemanager
- Implement stop command
- Use generic attributes in credential
- Allow export/import of identity
- Always require secure channel to authenticator
- Abstract over remote addresses with an alias system
- Cleaning up the alias configuration
- Genericise the node alias lookup system
- Simplify node configuration again
- Updated dependencies

### Fixed

- Clippy lints

### Removed

- Remove ability to set arbitrary attributes

## 0.11.0 - 2022-08-04

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Updated dependencies

## 0.9.0 - 2022-07-18

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service

## 0.8.0 - 2022-07-15

### Added

- Add `Identity` basic functionality to `ockam_api`
- Add schema validation tests for cloud api types
- Add tests for api cloud endpoints + fixes error handling

### Changed

- Use identity secure channels to communicate with orchestrator
- Extract common utils to process api services req/res/err
- Extract common utils to process api services req/res/err
- Move cloud api endpoints to run through the nodes service

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
