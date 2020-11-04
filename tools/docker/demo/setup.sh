#!/bin/bash

docker build . -f tools/docker/ockam-dev/Dockerfile -t ockam/ockam-dev
docker build . -f tools/docker/elixir/builder/Dockerfile
docker build . -f tools/docker/rust/Dockerfile.ockamd -t ockam/ockamd:0.1.0
docker build . -f tools/docker/rust/Dockerfile.router
docker build . -f tools/docker/influxdb/Dockerfile.influxdb-ockamd -t ockam/influxdb-ockamd:0.1.0
docker build . -f tools/docker/telegraf/Dockerfile.telegraf-ockamd -t ockam/telegraf-ockamd:0.1.0