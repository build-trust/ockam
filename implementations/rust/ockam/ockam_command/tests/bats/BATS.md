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

### Bats tests can also be run using our Builder Docker image

docker run --rm -it -e HOST_USER_ID=$(id -u) --volume $(pwd):/work ghcr.io/build-trust/ockam-builder:latest bash
bats implementations/rust/ockam/ockam_command/tests/bats

## How to run the unit tests

Unit tests doesn't need any special setup. This will run the simple local-only test:
```bash
bats implementations/rust/ockam/ockam_command/tests/bats
```

This will run all local-only tests, including the long ones:
```bash
LONG_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/bats
```

## How to run the orchestrator tests

The orchestrator tests require having an enrolled identity under `$OCKAM_HOME` (by default set at `$HOME/.ockam`), which will be copied to each test environment.

To do so, run the following:

```bash
ockam enroll
```

After this command completes, you will be able to run the orchestrator tests.

This will run the simple orchestrator tests that don't take too long:
```bash
ORCHESTRATOR_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/bats
```

This will run all orchestrator tests, including the long ones that can take several minutes to complete:
```bash
ORCHESTRATOR_TESTS=1 LONG_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/bats
```

## Executing the tests in parallel

The tests can be executed in parallel by using the `--jobs`/`-j` option. For example, to run the tests in 4 parallel jobs:

```bash
bats implementations/rust/ockam/ockam_command/tests/bats --jobs 4
```
