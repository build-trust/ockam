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


## Running Docs tests

To run docs tests, we need to setup the required environment

- Postgres server `docker run -d --name postgres -p 5432:5432 -e POSTGRES_PASSWORD=password postgres`
- Influx server `docker run --name influxdb -p 8086:8086 influxdb:2.7.4`
- Setup Influx, this requires [Nix](https://nixos.org/) `nix shell nixpkgs#influxdb2-cli --command influx setup --username username --password password --token token --org org --bucket bucket --name name --force`
- Setup required environment variables `export CONFLUENT_CLOUD_BOOTSTRAP_SERVER_ADDRESS="*********" CONFLUENT_CLOUD_KAFKA_CLUSTER_API_KEY="********" CONFLUENT_CLOUD_KAFKA_CLUSTER_API_SECRET="****" PG_HOST="127.0.0.1" INFLUX_PORT=8086 INFLUX_ORG=org INFLUX_BUCKET=bucket INFLUX_TOKEN=token`
- We currently support v3.7.0 and earlier, you can [download the Kafka client here](https://downloads.apache.org/kafka/3.7.0/kafka_2.13-3.7.0.tgz) and untar and set the required path, e.g., export PATH="$PATH_TO_KAFKA_BIN:$PATH"


To run the bats test, we use a [Nix shell](https://nixos.org/manual/nix/stable/command-ref/nix-shell)
```bash
nix develop --impure --expr 'let pkgs = import (builtins.getFlake "nixpkgs/nixos-23.11") {}; in pkgs.mkShell { buildInputs = with pkgs; [ postgresql python311Packages.psycopg2 python311Packages.flask telegraf ]; }' --command sh -c "BATS_TEST_RETRIES=2 ORCHESTRATOR_TESTS=1 DOCS_TESTS=1 bats ./implementations/rust/ockam/ockam_command/tests/bats/use_cases.bats"
```
