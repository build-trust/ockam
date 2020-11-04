![Ockam Logo](https://www.ockam.io/0dc9e19beab4d96b8350d09be78361df/logo_white_background_preview.svg)

# Ockam local Demo

## Getting started

### Prerequisites

- Unix based system (for running the shell scripts)
- Docker

### Automatic setup

Run `setup.sh`. This will do the manual steps for you.

### Setup (manual)

1. Build the builder environment<br>`docker build . -f tools/docker/ockam-dev/Dockerfile -t ockam/ockam-dev`
1. Build Elixir<br>`docker build . -f tools/docker/elixir/builder/Dockerfile`
1. Build Rust ockamd<br>`docker build . -f tools/docker/rust/Dockerfile.ockamd -t ockam/ockamd:0.1.0`
1. Build Rust router<br>`docker build . -f tools/docker/rust/Dockerfile.router`
1. Build influxdb-ockamd<br>`docker build . -f tools/docker/influxdb/Dockerfile.influxdb-ockamd -t ockam/influxdb-ockamd:0.1.0`
1. Build telegraf-ockamd<br>`docker build . -f tools/docker/telegraf/Dockerfile.telegraf-ockamd -t ockam/telegraf-ockamd:0.1.0`

### Run Demo use case

1. Run influxdb-ockamd <br>`./tools/docker/demo/influxdb.sh influxdb-ockamd`
1. Run telegraf-ockamd with the influxdb-ockamd public key as parameter <br>`./tools/docker/demo/influxdb.sh telegraf-ockamd <Public key>`
1. Send random temperature to telegraf-ockamd <br>`./tools/docker/demo/influxdb.sh telegraf-write`
1. Show influxdb temperature entries <br>`./tools/docker/demo/influxdb.sh influxdb-query`
1. Clean up <br>`./tools/docker/demo/influxdb.sh kill-all`

For more information see the `./tools/docker/demo/influxdb.sh` script help.
