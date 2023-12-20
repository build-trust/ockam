# Ockam

Thank you for your interest in contributing to the Ockam open source projects.

Ockam is a collection of protocols and toolkits for building connected systems that you can trust.
This folder contains the Swift implementation of the "Portals, by Ockam" application for MacOS.

Please read our community's [*Code of Conduct Covenant*][conduct] and our [contributing guidelines][contributing].

To start contributing to our Swift code, clone the Ockam repo from Github and change your current directory
to `implementations/swift`:

```
git clone git@github.com:build-trust/ockam.git
cd implementations/swift
```

# Using `make`

## Setup

If you don't already have it, you will need to [install XCode](https://developer.apple.com/xcode).

## Build

The Swift application implementation of the Portals application references some Rust code present in the `implementations/rust` directory.
In order to compile both the Rust code and the Swift code you can call:

```
make build
```

This produces an executable, which can be executed by typing:
```
"build/Ockam.xcarchive/Products/Applications/Portals, by Ockam.app/Contents/MacOS/Portals, by Ockam"
```

### Clean

The command to clean previously built artifacts is:
```
make clean
```

In some cases, if `make build` fails, you might have to both run `make clean` and recreate the `ockam/ockam_app/Ockam.xcodeproj`
directory by checking it out from git afresh.

### Release build

The previous command builds the debug version of the application. To build the release version you need to execute:
```
make build_release
```

## Test

The `make test` command exists but there are no automated tests at the moment.

## Lint

The `make lint` command exists but there are specified lints at the moment.

## Package

A `.dmg` file for the Portals application can be created by running:
```
make package
```

The `.dmg` file is created in the `build` directory.

## List of `make` commands

This table presents all the `make` commands, executed from the root directory

 Command                       | Description
 ------                        | -----------
 `make swift_clean`            | clean build files
 `make swift_build`            | build the application in debug mode
 `make swift_build_release`    | build the application in release mode
 `make swift_test`             | not implemented yet
 `make swift_lint`             | not implemented yet
 `make swift_package`          | create a `.dmg` file for the application

## Get Help

Ask a question on [Github Discussions](https://github.com/build-trust/ockam/discussions)


Note that these commands don't need to use the `rust_` prefix if you first `cd` into `implementations/rust`. Then you
can directly call: `make build`, `make test`, etc...

# Using XCode

We recommend using [XCode](https://developer.apple.com/xcode) to develop the project
and benefit from code navigation, refactoring and automated formatting.

Once XCode is installed, open the `implementations/swift/ockam/ockam_app/Ockam.xcodeproj` file to load the project.

[conduct]: https://github.com/build-trust/.github/blob/main/CODE_OF_CONDUCT.md
[contributing]: https://github.com/build-trust/.github/blob/main/CONTRIBUTING.md
