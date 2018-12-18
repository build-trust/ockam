# Contributing to Ockam

[![Discuss Ockam](https://img.shields.io/badge/slack-discuss-E01563.svg?logo=slack&style=flat-square)](https://join.slack.com/t/ockam-community/shared_invite/enQtNDk5Nzk2NDA2NDcxLWMzMzJlZjQzOTZjYWY0YmNkNWE1NmI1M2YyYzlkNjk4NDYyYzU0OWE0YTI4ZjcwNDBjNmQ4NzZjZTMzYmY3NDA)

Thank you for taking the time to contribute! :heart: :heart: :heart:

- [Ask a question](#ask-a-question)
- [Report an issue or a bug](#report-an-issue-or-a-bug)
- [Share an idea for a new feature](#share-an-idea-for-a-new-feature)
- [Contribute Code](#contribute-code)
	- [Development Environment](#development-environment)
	- [Build](#build)
	- [Lint](#lint)
	- [Test](#test)
	- [Project Conventions](#project-conventions)
		- [Spacing and Indentation](#spacing-and-indentation)
		- [Code Format](#code-format)
		- [Commit Messages](#commit-messages)
		- [Git Workflow](#git-workflow)
		- [Signed Commits](#signed-commits)
	- [Error Handling](#error-handling)
	- [Effective Go](#effective-go)
	- [Send a Pull Request](#send-a-pull-request)
- [Code of Conduct](#code-of-conduct)

## Ask a question
If you have a question about how Ockam works or how to use it, please
[join the Ockam Slack](https://join.slack.com/t/ockam-community/shared_invite/enQtNDk5Nzk2NDA2NDcxLWMzMzJlZjQzOTZjYWY0YmNkNWE1NmI1M2YyYzlkNjk4NDYyYzU0OWE0YTI4ZjcwNDBjNmQ4NzZjZTMzYmY3NDA)
to discuss with the Ockam team and community.

## Report an issue or a bug

If you’ve found a bug, please create an issue on [Github](https://github.com/ockam-network/ockam/issues/new). In the
issue description, please include:

* details on what you were expecting to happen
* what actually happened
* simple steps to reproduce what you observed ... with any code examples, if possible.
* details about the environment in which you were running ockam

## Share an idea for a new feature

If you have a new idea about how Ockam can be more useful, please create a feature request as a Github [issue](https://github.com/ockam-network/ockam/issues/new). Explain
the feature in detail, why it would be useful and how it will impact the Ockam codebase and community.

## Contribute Code
### Development Environment

The primary way to build Ockam code is by using the included `./build` bash script. This script requires recent
versions of Bash and Docker installed on your development machine.

Optionally, you can also work with Vagrant and Virtualbox installed on your development machine to create
a Debian virtual machine that includes Docker and Bash.

The `Vagrantfile` to create this virtual machine is included, run:

```
vagrant up && vagrant ssh
```

to work inside the Debian VM.

### Build

The primary way to build Ockam code is by using the included `./build` bash script.

The build script in turn uses Docker's multistage builds to create several tool images that always run the build
in a predictable, pinned environment and deliver reproducible builds.

To run a full build, run:

```
./build
```

This will download any dependencies in the `vendor` directory, create a `.build` directly and place compiled
binaries in that folder.

On OSX or Linux you can also run `./build install` to place your operating system specific `ockam` binary
in system path.

You can then run:

```
ockam --version
```

To see verbose output about what the build script is doing prefix `TRACE=1` before any build command, for example:

```
TRACE=1 ./build
```

For more options see `./build help`

### Lint

To run code linters, run:

```
./build lint
```

This will run all linters by default. Which includes eclint, commitlint, shellcheck and gometalinter.

For more options see `./build help lint`

### Test
To run tests and display test coverage, run:

```
./build test
```

### Project Conventions

#### Spacing and Indentation
Our spacing conventions are specified in our [editorconfig](.editorconfig) file and are enforced by `eclint` during
builds. Any files that do not comply will cause the build to fail. The easiest way to follow the conventions is to use
an [editorconfig plugin for your code editor](https://editorconfig.org/), most popular editors have one.

#### Code Format
All Go code must be formatted using [gofmt](https://golang.org/cmd/gofmt/). This is enforced by `gometalinter` during
builds. Any files that do not comply will cause the build to fail.

#### Commit Messages
All commit messages must follow
[Conventional Commits v1.0.0-beta.2](https://www.conventionalcommits.org/en/v1.0.0-beta.2/#summary). This is enforced
using `commitlint` during build. The exact rules that are enforced are specified in the
[commitlint.config.js](commitlint.config.js).

Allowed, type values:
* `build:` Changes that affect the build system or external dependencies
* `ci:` Changes to our CI configuration files and scripts
* `docs:` Documentation only changes
* `feat:` A new feature
* `fix:` A bug fix
* `perf:` A code change that improves performance
* `refactor:` A code change that neither fixes a bug nor adds a feature
* `style:` Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
* `test:` Adding missing tests or correcting existing tests

#### Git Workflow
We try to avoid noisy merge commits as much as possible and follow a
[rebase workflow](https://www.themoderncoder.com/a-better-git-workflow-with-rebase/) against our develop branch.
Please ensure that your pull requests can be rebased onto develop. See section below
[on pull requests](#send-a-pull-request).

#### Signed Commits
We only include [GPG Signed Commits](https://help.github.com/articles/signing-commits/). Please ensure that all
commits included in a pull request are signed with your GPG key.

### Error Handling
When dealing with errors returned within the code:

* Import `github.com/pkg/errors`
* When creating a new error use `errors.New("error message")`
* When dealing with an external error, that did not originate in Ockam's code, use `errors.WithStack(err)` to
add a stack trace to the error.
* always return errors and don't ignore them
* when printing errors, use fmt.Printf("%+v", err) to display the stack trace.
* never call `panic()`

### Effective Go

Use Go programming best practices defined in [Effective Go](https://golang.org/doc/effective_go.html)

### Send a Pull Request
When creating a pull request:

* Create a feature/bugfix branch from `ockam-network/ockam`’s **develop** branch.
* Add your changes to this branch.
* Send the pull request from your feature/bugfix branch. Don't send a pull from your master branch.
* Send the pull request against `ockam-network/ockam`’s **develop** branch.
* Make sure that there are no conflicts with develop and the code in your pull request can be rebased.
* Make sure all code convention described above are followed. Many of these are enforced by linter tools
that are called when you invoke `./build`.
* Makes sure the build succeeds `./build` with no linter warnings or test failures.

## Code of Conduct
All Ockam community interactions follow our [Code of Conduct](CODE_OF_CONDUCT.md). Please be polite and respectful
to all.

Thank you! :trophy: :tada: :confetti_ball:

— Ockam Team
