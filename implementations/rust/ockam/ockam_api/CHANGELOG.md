# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.83.0 - 2024-10-25

### Added

- Updated dependencies

### Fixed

- Use the proper tls configuration to export logs and traces

## 0.82.0 - 2024-10-24

### Added

- Pretty json output by default, and colored if possible
- Add more granular scopes for command logs
- Allow relay connection failure without failing relay creation
- Updated dependencies

## 0.81.0 - 2024-10-23

### Added

- Switching to sqlite wal mode for better concurrency
- Updated dependencies

### Fixed

- Auto retry `SQLite` queries and transactions when the failure is a deadlock
- Avoid panicking when the non-persistent node cannot be deleted

## 0.80.0 - 2024-10-21

### Added

- Change behavior of how nodes' processes are stopped
- Improvements to commands outputs
- Return enrollment ticket hex-encoded
- Updated dependencies

## 0.79.0 - 2024-10-16

### Added

- `eBPF` portal updates:
- Updated dependencies

### Fixed

- `project enroll` should not try to fetch project data from orchestrator

## 0.78.0 - 2024-10-15

### Added

- Update influxdb token lease manager response api
- More options on token management for influxdb_outlet
- Simplify influxdb outlet deployment options
- Compact enrollment ticket encoded format
- Implement a more efficient function to delete all project members at once
- Updated dependencies

### Fixed

- Generate the enrollment ticket using the project route, and not its id

## 0.77.0 - 2024-09-23

### Added

- Add a value parser for change histories
- Added `TLS` inlet support
- Implement influxdb token lessor service
- Update default rendezvous server address
- Influxdb inlet/outlet that attach authorization token
- Improve output for lease commands
- Refactor influxdb api client to better handle error responses
- Implementation of reliable `TCP` portals
- Add reliable `TCP` portals to `ockam_api`&`ockam_command`
- Improve ux of influxdb portal commands
- Updated dependencies

### Changed

- Bump opentelemetry-appender-tracing from 0.4.0 to 0.5.0
- Bump sysinfo from 0.30.13 to 0.31.4

### Fixed

- Graceful stop of a node in the command

## 0.76.0 - 2024-08-14

### Added

- Heavy kafka refactoring, moved portal interceptor from `api` to `tcp` crate
- Kafka cleanups
- Added the possibility to encrypt specific fields in a kafka `JSON` record
- Updated dependencies

## 0.75.0 - 2024-08-12

### Added

- Updated dependencies

### Changed

- Do not enforce the existence of project and authority identities

## 0.74.0 - 2024-08-06

### Added

- Rework `Session`s
- Crud for space and project admins
- The config of `node create` accepts an `identity`
- Updated dependencies

### Changed

- Move shared projects modifications logic into repository

### Fixed

- Make sure that there is only one space max marked as default
- When refreshing projects, store first the admin projects
- Use lowercase email in query filters
- Email list binding in project query
- Set default project/space/user

## 0.73.0 - 2024-07-29

### Added

- Add the possibility to configure the default client timeout
- Display the error when enrolling with a ticket fails
- Wait for the project to be ready before creating an authority client
- Set the default timeout on the background node client
- Don't interpret a bad request status as already enrolled
- Move rendezvous_server to `ockam rendezvous-server start`
- Implicitly resolve outlet addresses during connection
- Converted socket addresses to hostnames in command
- Remove sync operations
- Log commands by default to a file
- Hide actual values for `Token` and `OneTimeCode` in `Debug`
- Increase opentelemetry queue sizes
- Adjust timeouts
- Report more detailed errors
- Stop a previous medic before starting a new one
- Integrate space's subscription data in command
- Handle duplicates in project's egress_allow_list field
- Updated dependencies

### Changed

- Always log messages from the terminal if logging is true

### Fixed

- Make sure that only one project is the default one

## 0.72.0 - 2024-07-03

### Added

- Updated dependencies

### Changed

- Use a published dependency for the patched sqlx library
- Improve output of `project enroll` and `credential` commands

## 0.71.0 - 2024-07-01

### Added

- Improve transport imports
- Integrate `UDP` puncture into `ockam_api`
- Add delete and list commands for kafka-outlet
- Use the any driver for sqlx to add support for postgres
- Change tcp protocol serialization
- Optimize cbor encoding by preallocating memory
- Updated dependencies

### Changed

- `project-member` commands, and adds the `show` command
- `kafka-*` commands

## 0.70.0 - 2024-06-25

### Added

- Add `identity` arg to `tcp-inlet create` to customize secure channel identifier
- Add `disable-content-encryption` flag to the kafka-inlet create command
- Exposed and added `ockam-rely` attribute validation for relay service
- Unified relay creation logic for project and rust
- Adapt the width of separator lines depending on the terminal width
- Updated dependencies

### Fixed

- Do not create instance of `HighlightLines` struct to prevent unexpected behaviors
- Ping directly the other node in relay rather than self-ping
- Terminal width detection, which was returning invalid values on ci

## 0.69.0 - 2024-06-11

### Added

- Kafka debugging of initial `ApiVersions`, useful when connection is closed right away
- Improve output for `project show` command
- Add jq filtering to commands json output
- Updated dependencies

### Changed

- Bump opentelemetry-appender-tracing from 0.3.0 to 0.4.0

### Fixed

- Do not add a newline on command output when stdout is not a tty

## 0.68.0 - 2024-05-30

### Added

- Create a portal for exporting traces when a project exists
- Show the unauthorized identifier in the logs
- Print the exact error when a persisted secure channel cannot be retrieved
- Make `default` vault reuse `SqlxDatabase` instance
- Fixed tls tcp outlets and kafka outlets
- Updated dependencies

## 0.67.0 - 2024-05-28

### Added

- Improve output of `project enroll` command
- Improve output of `node show` and `status` commands
- Create a project member for exporting traces when the authority node starts
- Create and store a default project when starting an authority node
- Add more log messages
- Add the possibility to use boolean expressions for policy expressions
- Address review comments
- Implement updating route to the outlet in the existing inlet
- Add an http server to the node manager to return the node resources
- Improvements for commands' output to standardize their formatting
- Removed consumer/producer/direct services and added inlet service
- Introduced consumer resolution and publishing concepts and implementation
- Added abac rules to kafka inlet and oulet
- Introduce granular ac for kafka portal worker
- Added policy access control usage
- Introducing a variant of the secure channel which only exchange keys
- Using key exchanger in kafka secure channel map
- Allow kafka portals to anchor trust on identities
- Switch to standard relay creation for kafka usage
- Use a different logger to log tracing/logging errors
- Add secure channel persistence
- Add secure channel persistence to kafka
- Updated dependencies

### Changed

- Upgrade the rust version to 1.77

### Fixed

- Printing multi-line logs generated by commands stdout output
- Allow initial credential exchange for key exchange only
- Fix outgoing policy in kafka outlet

## 0.66.0 - 2024-04-30

### Added

- Improve output of `node show` command
- If logging is enabled, command output will be redirected to the logs
- Improve output of `node create` command
- Updated dependencies

## 0.65.0 - 2024-04-23

### Added

- Support https for outlets
- Export opentelemetry traces by default
- Make the api for creating outlets more flexible
- Support progress_bar in command notifications
- Improve output of `node create` command
- Scope some repositories to a given node name
- When deleting a node, wait for node's process to finish
- Updated dependencies

### Fixed

- Set the global error handler even if logging is off

### Removed

- Removed an empty file

## 0.64.0 - 2024-04-12

### Added

- Add a attribute with the content of a node configuration file
- Add a user journey event when an identity has been created or imported
- Use outgoing access control
- Added kafka-inlet command and relative config side
- Updated dependencies

### Changed

- Organize bats tests in different suites
- Move terminal code from command to api

### Fixed

- Kms identity can be used in regular api nodes

## 0.63.0 - 2024-04-01

### Added

- Authority project admin credentials
- Admins are implicit members, enrollers as admins
- `identity create` can import an identity
- Backcompatible encoding/decoding optimizations
- Improve output for `enroll` command
- Add one second cache for incoming and outgoing access control
- Flag to enable/disable enrollers-as-admins on authority
- Use https for the default opentelemetry collector endpoint
- Add bats coverage for `node create ./config.yaml` command
- Reply to v1 transport messages with v1 transport messages
- Enable the tracing context on the rust side
- Store enrollment email to local db
- Create 3 separate credential retriever types
- Introduce `disable_trust_context_id` argument for authority
- Updated dependencies

### Changed

- Simplify `ProgressDisplay` to remove the mutex used to stop the message recv end

### Fixed

- Fix routing and flow control for local kafka outlets

## 0.62.0 - 2024-03-25

### Added

- Authority project admin credentials
- Admins are implicit members, enrollers as admins
- `identity create` can import an identity
- Backcompatible encoding/decoding optimizations
- Improve output for `enroll` command
- Add one second cache for incoming and outgoing access control
- Flag to enable/disable enrollers-as-admins on authority
- Use https for the default opentelemetry collector endpoint
- Add bats coverage for `node create ./config.yaml` command
- Updated dependencies

### Changed

- Simplify `ProgressDisplay` to remove the mutex used to stop the message recv end

### Fixed

- Fix routing and flow control for local kafka outlets

## 0.61.0 - 2024-03-18

### Added

- Tune the timeouts for checking if a node is ready
- Added manual tests to measure latency
- Upgraded kafka library, with kafka 3.7.0 support
- Propagating the errors from api clients to the command
- Add the node name to spans
- Instrument the tcp portal
- Instrument more functions for secure channels
- Start a new trace before sending a transport message
- Update display, log output in frequently used commands
- Introduced several cpu consumption optimizations
- Add an environment variable to specify if a user is an ockam developer
- Updated dependencies

### Changed

- Don't initialize logging at all if log is not enabled
- Rename methods and variables to insist on the exporting
- Refactor the code thanks to pr review comments
- Do small renaming of some local variables

### Fixed

- Fix the blocking processing of spans and log records
- Fix the creation of a trace id from a project id in tests

### Removed

- Remove resources when deleting a node

## 0.60.0 - 2024-02-28

### Added

- Add support for additional kafka addons
- Improve ockam enroll command ux output, help, logs, errors
- Add opentelemetry tracing and logging support
- Allow running `reset` command even if the database is in an invalid state
- Restart a project journey if project is deleted
- Delete `TrustContext`
- Add `skip_is_running_check` to the authority node
- Add application errors
- Improve ockam tcp-outlet commands ux output, help, logs, errors
- Improve credentials management
- Backup logs when app restarts inlet node
- Address review comments
- Instrument more functions for enrollement
- Simplifies `projects` section from the `run` config file
- Introduce `subject.has_credential`
- Unify creation and retry connection for portal and relay
- Improve authority debug-ability
- Tcp inlet creation will always optional validate unless `--no-connection-wait` is used
- Add `--force` flag to `enroll` command and switch default behavior
- Pass the tracing context at the ockam message level
- Add policies for resource types
- Improve portals reliability and integration tests
- Add an environment variable to configure a crates filter for log messages
- Create time-limited journeys
- Hash the host name used in the trace id
- Refactor `Project`-related code
- Update enroll ux with new help text, display, and log progress status messages
- Start a new trace for a background node
- Rework migrations
- Updated dependencies

### Changed

- Move the handling of attributes expiration date to a layer above the repository
- Separate transport messages from local messages
- Enable tracing by default
- Incorporate review comments
- Extract the progress display as a separate struct
- Get the default project only once

### Fixed

- Fix clippy warnings on nightly
- Close the context automatically on each test macro execution
- Execute logging / tracing tests as integration tests
- Command's verbose argument now has preference over env vars
- Store policies isolated by node and resource
- Make the journeys test more robust
- Fix okta authenticator, add identities to members table
- Set the proper span id on the propagated tracing context
- Use a stable span name for the root span of the host journey
- Avoid leaking resources when one step of the cleanup fails
- Use the correct policies in inlets/outlets created by kafka services
- Policy bats tests
- Fixed flaky kafka integration test
- Fixed kafka-related flaky tests
- Put the tracing context field under a compilation flag
- Avoid triggering tokio invalid reference drop in test
- Disable portal packet counter field
- Do not enforce enrollment limit
- Do not log messages by default on command parsing errors
- Don't set a logging appender when logging is disabled
- Fix a sql query
- Get project identifier from model, without building the whole identity
- Use project auth identifier in the journey instead of identity
- Fix the flushing of traces
- Make the journeys test more robust

### Removed

- Remove the tracing of sensitive parameters
- Remove `--resource` and `--resource-type` args from `policy show|list|delete`
- Remove some unnecessary context stops

## 0.59.0 - 2024-01-09

### Added

- Use `From` for converting errors
- Make authority issued credentials ttl configurable
- Updated dependencies

## 0.58.0 - 2024-01-04

### Added

- Updated dependencies

## 0.57.0 - 2023-12-26

### Changed

- Close unneeded tcp connections in various clients
- Updated dependencies

## 0.56.0 - 2023-12-19

### Changed

- Updated dependencies

## 0.55.0 - 2023-12-18

### Changed

- Updated dependencies

## 0.54.0 - 2023-12-16

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Persist application data in a database
- Slim down the node manager worker(s_ch)
- Updated dependencies

### Fixed

- Don't create default node when retrieving it and doesn't exist

### Removed

- Remove recursive calls in repository implementations

## 0.53.0 - 2023-12-15

### Changed

- Updated dependencies

## 0.52.0 - 2023-12-12

### Changed

- Updated dependencies

## 0.51.0 - 2023-12-11

### Changed

- Slim down the node manager worker(s_ch)
- Updated dependencies

## 0.50.0 - 2023-12-06

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Persist application data in a database
- Updated dependencies

### Fixed

- Don't create default node when retrieving it and doesn't exist

### Removed

- Remove recursive calls in repository implementations

## 0.49.0 - 2023-12-05

### Added

- Add `VersionedData::data_type`. remove hash truncation

### Changed

- Persist application data in a database
- Updated dependencies

### Removed

- Remove recursive calls in repository implementations

## 0.48.0 - 2023-11-23

### Changed

- Use `Identifier` as a return type in public api
- Updated dependencies

## 0.47.0 - 2023-11-17

### Changed

- Use `Identifier` as a return type in public api
- Updated dependencies

## 0.46.0 - 2023-11-08

### Changed

- Always using enum when representing the inlet connection status
- Updated dependencies

## 0.45.0 - 2023-11-08

### Changed

- Always using enum when representing the inlet connection status
- Updated dependencies

## 0.44.0 - 2023-11-02

### Changed

- Setup app's logs with the same features we use in the cli
- Updated dependencies

## 0.43.0 - 2023-10-26

### Changed

- Updated dependencies

## 0.42.0 - 2023-10-25

### Changed

- Updated dependencies

## 0.41.0 - 2023-10-18

### Changed

- Updated dependencies

## 0.40.0 - 2023-10-07

### Changed

- Make `Timestamp` arithmetic operations usage safer
- Cli's `random_name` function now returns human-readable two-word strings like 'fit-lark'
- Move the controller address to the node manager
- Use better names for request / response headers
- Introduce a secure client for the controller
- Use controller, authority and project nodes
- Simplify connections
- Introduce a supervised node manager to support connection replacements
- Adjust the code after rebase
- Move the in memory node to the ockam api crate
- Package all reply / response methods into a client
- Use the client in the background node
- Put back the is_rust check to create forwarders
- Rename forwarder to relay
- Updated dependencies

### Fixed

- Fix the sending of messages
- Fix the code after rebasing
- Drop the in memory node and delete its node manager

### Removed

- Remove an unused method
- Remove the need to keep a flag to skip defaults
- Remove two parameters from requests to the controller
- Remove the unused tag feature
- Remove the unused rpc proxy service
- Remove the supervised node manager
- Remove the secure clients struct

## 0.39.0 - 2023-10-05

### Changed

- Make `Timestamp` arithmetic operations usage safer
- Cli's `random_name` function now returns human-readable two-word strings like 'fit-lark'
- Move the controller address to the node manager
- Use better names for request / response headers
- Introduce a secure client for the controller
- Use controller, authority and project nodes
- Simplify connections
- Introduce a supervised node manager to support connection replacements
- Adjust the code after rebase
- Move the in memory node to the ockam api crate
- Package all reply / response methods into a client
- Use the client in the background node
- Put back the is_rust check to create forwarders
- Rename forwarder to relay
- Updated dependencies

### Fixed

- Fix the sending of messages
- Fix the code after rebasing

### Removed

- Remove an unused method
- Remove the need to keep a flag to skip defaults
- Remove two parameters from requests to the controller
- Remove the unused tag feature
- Remove the unused rpc proxy service
- Remove the supervised node manager
- Remove the secure clients struct

## 0.38.0 - 2023-09-28

### Added

- Add authority tests

### Changed

- Move authority node code level above in `ockam_api`
- Break up authenticator
- Updated dependencies

### Fixed

- Reset cli state if it can't be parsed

### Removed

- Remove scopes for authority members

## 0.37.0 - 2023-09-23

### Changed

- Switch to new `Identity` design
- Adapt to new identity design
- Updated dependencies

## 0.36.0 - 2023-09-22

### Changed

- Switch to new `Identity` design
- Adapt to new identity design
- Updated dependencies

## 0.35.0 - 2023-09-13

### Changed

- Updated dependencies

## 0.34.0 - 2023-09-06

### Added

- Added a direct local kafka for simple deployments and fixed service registry

### Changed

- Improve tcp disconnect api
- Use proper url data type
- Create a relay to the default project after enrolling and when starting the app
- Move common code to `api` so we can remove `command` from `app`
- Updated dependencies

### Fixed

- Fix the cbor annotations for non-borrowed data

### Removed

- Removed api lifetimes to access node manager operations directly
- Remove the `projects` field from `NodeManager` to load them from the `CliState`

## 0.33.0 - 2023-06-26

### Added

- Add more meaningful error messages for `CLiState` errors

### Changed

- Improve type safety for `FlowControls`
- Hide `Spawner` vs `Producer` logic under the hood
- Replace `crate::Result` with `miette::Result` as the main result type on command
- Update ockam api services error responses to using a struct
- Updated dependencies

## 0.32.0 - 2023-06-09

### Added

- Add more information about which processes use which files
- Add delete and list subcommands for kafka consumer/producer commands

### Changed

- Document the layout of files for a node
- Extend direct authenticator service to list and delete members
- Make `AccessControl` optional while starting a `Worker`
- Full local kafka implementation which credential validation and flow control
- Updated dependencies

### Removed

- Remove old config.json file and add migration

## 0.31.0 - 2023-05-26

### Added

- Add unit tests for the node and identity initialization

### Changed

- Rename import identity to decode identity since it is not importing anything
- Introduce a retrieve identity function returning an option
- Use identity identifiers for the creation of secure channels
- Use identity identifier for credentials
- Use an identity identifier for the node manager worker in kafka
- Use an identity identifier for the authority service
- Use a key value file storage for the vault
- Extract the vault_aws crate
- Simplify the identity state config
- Migrate the identities configuration
- Migrate only item paths
- Initialize the default node outside of the command run impl
- Move `FlowControls` to `Context` and make it mandatory
- Make `FlowControl` more mistake-resistant
- Improve `RpcProxyService`
- Improve `TCP` `::connect()` and `::listen()` outputs
- Improve `::create_secure_channel()` and `::create_secure_channel_listener()` output
- Improve tcp command ux
- Updated dependencies

### Removed

- Remove the need for a state item to know about the global state
- Remove unneeded `FlowControls` instance from `Auth API`

## 0.30.0 - 2023-05-12

### Changed

- Updated dependencies

### Removed

- Remove the vault service which is not used

## 0.29.0 - 2023-05-04

### Added

- Added a readme template and updated some readmes

### Changed

- Apply cli_state abstraction to identities and projects
- Apply cli_state abstraction to credentials and trust_contexts
- Apply cli_state abstraction to nodes
- Authority node creation
- Updated dependencies

### Fixed

- Move to the smaller, cargo-team maintained `home` crate
- Fix docs build for api and multiaddr crates

## 0.28.0 - 2023-04-27

### Changed

- Create a default project policy for a tcp inlet/outlet
- Extract identity as an entity
- Moved the builder functions to their respective structs
- Formatting
- Move the lmdb storage
- Ockam enroll outputs a ticket containing code and project
- Create abstraction for the cli state directories and applies it to the vaults state
- Allow kafka reconnection when project connection goes down
- Use the tcp constant for the transport type
- Updated dependencies

### Fixed

- Do not recreate an identity state if it already exists
- Resolve transport addresses as a separate step

### Removed

- Remove the vault service endpoint for getting secret data
- Removed the put_identity function on identities writer

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
- 'ockam enroll' overwrites current configuration instead of returning error
- Update cli_state test with credentials entry

## 0.24.0 - 2023-02-24

### Added

- Add default subcommand to node

### Changed

- Pre-trusted identity identifiers attributes
- Use credential instead of credentials
- Usable kafka sidecar implementation
- Standardize where authority stores membership information
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
