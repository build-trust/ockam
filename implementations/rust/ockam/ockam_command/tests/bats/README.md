## Install

- https://bats-core.readthedocs.io/en/stable/
- https://github.com/ztombol/bats-docs#installation
- https://github.com/ztombol/bats-assert

MacOS:

```bash
brew tap kaos/shell
brew install bats-assert
```

Linux:
```bash
npm install -g bats bats-support bats-assert
```

### How to format the tests scripts

We use the `shfmt` tool, which can be download from https://github.com/mvdan/sh.

```bash
shfmt -w ./implementations/rust/ockam/ockam_command/tests/bats/
```

### Bats tests can also be run using our Builder Docker image

docker run --rm -it -e HOST_USER_ID=$(id -u) --volume $(pwd):/work ghcr.io/build-trust/ockam-builder:latest bash
bats implementations/rust/ockam/ockam_command/tests/bats

## How to run the unit tests

Unit tests doesn't need any special setup. This will run the simple local-only test:
```bash
bats implementations/rust/ockam/ockam_command/tests/bats
```

_note_: In addition to setting `$OCKAM` executable's path, `$BATS_LIB` must be set the directory you have installed `bats-support` and `bats-assert`.

## How to run the orchestrator tests

The orchestrator tests require having an enrolled identity under `$OCKAM_HOME` (by default set at `$HOME/.ockam`), which will be copied to each test environment.

To do so, run the following:

```bash
ockam enroll
```

After this command completes, you will be able to run the orchestrator tests.

This will run the simple orchestrator tests:
```bash
ORCHESTRATOR_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/bats
```

## Executing the tests in parallel

The tests can be executed in parallel by using the `--jobs`/`-j` option. For example, to run the tests in 4 parallel jobs:

```bash
bats implementations/rust/ockam/ockam_command/tests/bats --jobs 4
```
