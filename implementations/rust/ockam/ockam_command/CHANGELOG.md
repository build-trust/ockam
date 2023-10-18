# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.98.0 - 2023-10-18

### Changed

- Drop dependency on termcolor for ockam_command crate
- Updated dependencies

## 0.97.0 - 2023-10-07

### Changed

- Move the controller address to the node manager
- Implement subscriptions directly on the node manager
- Start the node manager worker for remaining rpc calls
- Extract an interface for subscriptions
- Use better names for request / response headers
- Use a more precise interface for the subscriptions trait
- Introduce a secure client for the controller
- Use a secure client to enroll
- Use controller, authority and project nodes
- Simplify the creation of a local node
- Move the secure client close to secure channels
- Reduce the dependencies of rpc
- Move the rpc to ockam api as remote node
- Rename local/remote node to in memory/background
- Use only cli state to create a background node
- Move the in memory node to the ockam api crate
- Make the use of the controller client more explicit
- Package all reply / response methods into a client
- Use the client in the background node
- Improve help of global verbose flag in ockam command
- Improve `feedback` section of the `help` text
- Improve cli "learn more" section from the help text
- Updated dependencies

### Fixed

- Breaking changes in upgrading dialoguer crate to 0.11.0
- Fix a test argument
- Drop the in memory node and delete its node manager

### Removed

- Remove the unused tag feature
- Remove the supervised node manager
- Remove the secure clients struct

## 0.96.0 - 2023-10-05

### Changed

- Move the controller address to the node manager
- Implement subscriptions directly on the node manager
- Start the node manager worker for remaining rpc calls
- Extract an interface for subscriptions
- Use better names for request / response headers
- Use a more precise interface for the subscriptions trait
- Introduce a secure client for the controller
- Use a secure client to enroll
- Use controller, authority and project nodes
- Simplify the creation of a local node
- Move the secure client close to secure channels
- Reduce the dependencies of rpc
- Move the rpc to ockam api as remote node
- Rename local/remote node to in memory/background
- Use only cli state to create a background node
- Move the in memory node to the ockam api crate
- Make the use of the controller client more explicit
- Package all reply / response methods into a client
- Use the client in the background node
- Improve help of global verbose flag in ockam command
- Improve `feedback` section of the `help` text
- Improve cli "learn more" section from the help text
- Updated dependencies

### Fixed

- Breaking changes in upgrading dialoguer crate to 0.11.0
- Fix a test argument

### Removed

- Remove the unused tag feature
- Remove the supervised node manager
- Remove the secure clients struct

## 0.95.0 - 2023-09-28

### Changed

- Updated dependencies

### Fixed

- Reset cli state if it can't be parsed
- Reset cli state if it can't be parsed
- `ockam status` now works without an existing identity

## 0.94.0 - 2023-09-23

### Added

- Add installation instructions for ockam command

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.93.0 - 2023-09-22

### Added

- Add installation instructions for ockam command

### Changed

- Switch to new `Identity` design
- Updated dependencies

## 0.92.0 - 2023-09-13

### Changed

- Extract the output of request results from the rpc code
- Replace most rpc new calls with either embedded or background
- Updated dependencies

### Removed

- Remove the lifetime annotations for rpc

## 0.91.0 - 2023-09-06

### Added

- Add flag to control whether a node redirects the logs to a file
- Add support to create and list tcp-outlets on the desktop app
- Add cli subcommand to list share invitations
- Added new cli command to retrieve the project's version
- Added a direct local kafka for simple deployments and fixed service registry

### Changed

- Foreground nodes always log to stdout
- Scaffold for ockam_app with support for enroll
- Formatting
- Prototype command to check orchestrator nodes versions
- Load controller address and identifier from env
- Integrate orchestrator versions within the status command
- Introduce an app state holding a context
- Change some response functions
- Optionally share service when creating the tcp-outlet
- New sidecar to run inlet/outlet relay portal with one command
- Simplify tcp-inlet creation
- Move common code to `api` so we can remove `command` from `app`
- Updated dependencies

### Fixed

- Foreground nodes will write logs to file on a fresh start
- Fix compilation errors
- Read default values for `GlobalArgs` from env variables

### Removed

- Removed api lifetimes to access node manager operations directly

## 0.90.0 - 2023-06-26

### Added

- Add more meaningful error messages for `CLiState` errors
- Add "preview" tag to commands that are in developer preview

### Changed

- Improve error messages returned when parsing the node name argument
- Replace `crate::Result` with `miette::Result` as the main result type on command
- Updated dependencies

### Fixed

- Fix and simplify cli pager used to display help texts

## 0.89.0 - 2023-06-09

### Added

- Add standard list output and implement for all list commands

### Changed

- Use async configure addon endpoint
- Improve error definitions within ockam command and error handling within ockam enroll
- Paginate help texts
- Full local kafka implementation which credential validation and flow control
- Updated dependencies

### Fixed

- Fix test where the real `CliState` was being used instead of an isolated instance

### Removed

- Remove old config.json file and add migration
- Remove usage of chrono, fix clippy issues

## 0.88.0 - 2023-05-26

### Added

- Add unit tests for the node and identity initialization
- Add `ockam flow-control add-consumer FLOW_CONTROL_ID ADDRESS` command

### Changed

- Use an identity identifier for node details
- Use an identity identifier for the node manager worker in kafka
- Simplify the identity state config
- Migrate the identities configuration
- Initialize the default node outside of the command run impl
- Environment command & moved text
- Improve on text and outputs of enroll, influx and kafka commands
- Move `FlowControls` to `Context` and make it mandatory
- Update cli manual docs some commands
- Updated dependencies

### Fixed

- Fix the formatting
- Rename ockam `project authenticate` clap command to ockam `project enroll`

## 0.87.0 - 2023-05-12

### Added

- Add spacing around header

### Changed

- Update enroll output add ascii
- Tweak the formatting of fmt macros
- Clean up color usage, touch up progress bar
- Move displaying of argument parsing logs
- Improve on text and outputs of enroll, influx and kafka commands
- Updated dependencies

### Fixed

- Fix clippy linter issue

## 0.86.0 - 2023-05-04

### Added

- Add all available environment variables to the displayed in commands help text
- Added a readme template and updated some readmes

### Changed

- Apply cli_state abstraction to identities and projects
- Apply cli_state abstraction to credentials and trust_contexts
- Apply cli_state abstraction to nodes
- Store serialized identity in the config instead of storing in parts
- Rotate cli logs
- Update how we handle user confirmation on `reset` command
- Use 'local ockam configuration' in messages instead of cli state
- Automate the creation and update of readmes
- Updated dependencies

### Fixed

- Parsing `GlobalArgs` from input
- Move to the smaller, cargo-team maintained `home` crate
- On `reset` command, don't prompt the user if `-y` flag is passed

## 0.85.0 - 2023-04-27

### Added

- Add new line to end of fixture file
- Add new output formats to create/default/delete vault commands

### Changed

- Rename ockam forwarder commands to ockam relay
- Extract identity as an entity
- Improve outputs of tcp outlet, inlet and relay
- Cli docs to handle fourth level markdown headers
- Create standalone commands for kafka services
- Updated dependencies

### Fixed

- Update test referencing ockam forwarder
- Fix linter issues
- Fix other clippy linter issues
- Fix argument unit test for project authenticate
- Return err instead of expect, move enrollment ticket to fixture

## 0.84.0 - 2023-04-14

### Added

- Add a limited version of the `ockam run` command

### Changed

- Implement custom get_env
- Update commands that use project path to also accept trust context
- Improve command help
- Rename `Sessions` -> `FlowControls`
- Use cli state for trust context and default trust context
- Create a dbackground default node on demand
- Updated dependencies

### Fixed

- Fix project deletion from state
- Fix `authenticated` command & `Sessions`

## 0.83.0 - 2023-03-28

### Added

- Add shell abstraction to handle commands output streams
- Add a command to create an authority node
- Add examples and about sections to markdown generated docs
- Add basic documentation for node, identity and space commands

### Changed

- Use tcp session on authenticated command
- Refactor the calls to the syntax highlight function
- Updated dependencies

### Fixed

- Improve markdown help renderer

### Removed

- Remove warnings
- Removed type parameters exposing implementation details

## 0.82.0 - 2023-03-03

### Changed

- Refactor `CliState` so it can be built using an explicit directory
- Parse `/node/n1` to `/worker/addr` after connecting to the node via tcp
- Update `authenticated` command tcp
- Use abac in authority services implementation
- Expand credential commands
- Updated dependencies

## 0.81.0 - 2023-02-24

### Changed

- Move the `OneTimeCode` struct from the ockam_api crate to the ockam_identity crate
- Pre-trusted identity identifiers attributes
- All functions from ockam_command now return a `crate::Result`
- Updated dependencies

### Fixed

- Reduce cli bootstrap time by an order of magnitude for both release and dev profiles
- Commands shows concise errors with a more human-readable format
- Update project readiness check to include authority

### Removed

- Remove the lifetime annotation on `Credential` and `Attributes`

## 0.80.0 - 2023-02-09

### Changed

- Updated dependencies

### Fixed

- Apply `clippy --fix`

## 0.79.0 - 2023-01-31

### Added

- Add influxdb lease commands, orchestrator client, and default project
- Add worker list command
- Support cloud opts project on all orchestrator commands
- Add support for starting an embedded node with project info optionally

### Changed

- Move `storage` and `registry` to `Identity`
- Refactor `CliState` so the `authenticated_storage` is stored in the identities dir
- Moved optional `identity_name` to higher level `cloudrequestwrapper` struct
- Extract large strings into constants directory
- Reorder subcommands to match enum
- On `ockam enroll`, enroll the admin as a member of all their projects
- Always enforce-credentials on cli
- Updated dependencies

### Fixed

- Fix errors in ockam status command
- Self enroll admin as a project member when creating a project

## 0.77.0 - 2022-11-08

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples
- Add message a space being created is trail space
- Add subscription commands
- Add `reset` command, refactor `node delete`
- Add `--config` argument to `node create` command
- Add addons commands
- Add okta auth command
- Add policy command
- Add command to list policies of a resource
- Add okta config validation on addon configuration

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Refactor rpc struct to allow working with embedded nodes
- Use embedded nodes as default on commands
- Derive identity identifier from identity
- Minor refactors to commands/api error handling
- Display a message if a new version of command is available
- Improve mechanism for command upgrade message
- Flag when creating project to enforce credentials true|false
- Move admin-only subscription commands under `ockam admin` command
- Upgrade ockam_command to clap v4
- Recover remote forwarder
- Upgrade to `clap v4` release version
- Extend the declarative config support
- Unify ockam_command help
- `node create` to return result
- `node start` reads from the config file to execute the appropriate commands
- Okta identity provider
- Enforce certificate pinning on okta tenants
- Reduce output for short help command
- Complete policy delete functionality
- Make the okta tenant config more generic
- Make handle_message default value of action in policy set
- Hide command export arguments from help
- Eagerly get membership credential
- Waits until project is ready after okta plugin is enabled
- Show ockam_command version when printing an error
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name
- Project enroll
- Project info is persisted properly
- Show help output when no args passed
- Auth0 error message text when failing to validate provider config

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands
- Remove node arg from enroll command

## 0.76.0 - 2022-09-21

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples
- Add message a space being created is trail space
- Add subscription commands
- Add `reset` command, refactor `node delete`

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Refactor rpc struct to allow working with embedded nodes
- Use embedded nodes as default on commands
- Derive identity identifier from identity
- Minor refactors to commands/api error handling
- Display a message if a new version of command is available
- Improve mechanism for command upgrade message
- Flag when creating project to enforce credentials true|false
- Move admin-only subscription commands under `ockam admin` command
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name
- Project enroll

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands
- Remove node arg from enroll command

## 0.75.0 - 2022-09-09

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples
- Add message a space being created is trail space

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Refactor rpc struct to allow working with embedded nodes
- Use embedded nodes as default on commands
- Derive identity identifier from identity
- Minor refactors to commands/api error handling
- Display a message if a new version of command is available
- Improve mechanism for command upgrade message
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name
- Project enroll

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands
- Remove node arg from enroll command

## 0.74.0 - 2022-09-07

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples
- Add message a space being created is trail space

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Refactor rpc struct to allow working with embedded nodes
- Use embedded nodes as default on commands
- Derive identity identifier from identity
- Minor refactors to commands/api error handling
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name
- Project enroll

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands

## 0.73.0 - 2022-09-05

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Refactor rpc struct to allow working with embedded nodes
- Use embedded nodes as default on commands
- Derive identity identifier from identity
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name
- Project enroll

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands

## 0.72.0 - 2022-08-31

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Refactor rpc struct so it allows working with embedded nodes
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands

## 0.71.0 - 2022-08-29

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages
- Add authority command
- Add shell completions command
- Add credentials commands
- Add syntax highlighting to shell script examples

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Make it easier to write commands' api req/res handlers
- Replace signer with verifier
- Simplify "sc listener list" command
- Create default space, project and secure channel after enrolling
- Get rid of old `ockam_command` module
- Improve secure channel commands
- Unhide the enroll command
- Create projects' secure channels present in the input multiaddr
- Change `--from` argument of forwarder create to `FORWARDER_NAME`
- Make `embedded_node` stop node itself
- Highlight examples with different colors based on terminal background
- Use names instead of ids on spaces and projects commands
- Restructure ockam command modules and improve help
- Group global options in command help
- Make version a local toplevel option in ockam command
- Improve ockam command help
- Improve ockam command help
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs
- Mutliaddr support for projects
- Creation of static forwarder at local nodes
- Display the correct syntax theme base on `COLORFDBG`
- Node creation without a name

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes
- Remove email enroll and enrollment token commands

## 0.70.0 - 2022-08-17

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Change portal sub command to tcp-inlet and tcp-outlet
- Change `forwarder create` command arguments to --for and --at
- Unhide the forwarder subcommand
- Improve command help with examples
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes

## 0.69.0 - 2022-08-12

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages
- Add `SHOW_HIDDEN` environment variable
- Add api endpoint to send messages

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Split `transport` into `tcp-connection` and `tcp-listener`
- Long_version should display git hash
- Hide identity create and vault from command help
- Basic alias system
- Re-hide alias command
- Rename alias to configuration
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed
- Replace args containing `-/` or `/-` with stdin
- Fix link to command line docs

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command
- Remove invitations code
- Remove ability to set arbitrary attributes

## 0.68.0 - 2022-08-04

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg
- Add service command
- Add argument tests for `node show` and `node delete`
- Add global command to disable ansi colors on tracing messages

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes
- Use temporary secure channel on cloud and enroll api endpoints
- Command config updates
- Rename `ockam config` to `ockam configuration`
- Adapt cli commands
- Rename `-a, --api-node` option to `-n, --node`
- Rename ockam command output format option
- Split `SecureChannel` into `Self` and `SecureChannelListener`
- Updated dependencies

### Fixed

- `addr` argument for cloud commands
- Cloud and node arguments set as global
- `project create` command now works when services + node + cloud args are passed
- Space create command when list args are passed

### Removed

- Remove custom validator on authenticated command
- Remove short flag `-f` for `--format` global option in command

## 0.67.0 - 2022-07-18

### Fixed

- `addr` argument for cloud commands

## 0.66.0 - 2022-07-18

### Added

- Add basic `Identity` commands to `ockam_command`
- Add `message-format` global arg

### Changed

- Use same variable names on some ockam_command commands
- Cloud commands to send requests through nodes
- Send cloud node address from cloud commands to nodes

### Fixed

- `addr` argument for cloud commands

### Removed

- Remove custom validator on authenticated command

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

