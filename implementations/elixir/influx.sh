#!/usr/bin/env bash

set -e
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
BUILD_DIR="${SCRIPT_DIR}/_build"
VAR_DIR="${BUILD_DIR}/influxdb"

mkdir -p "${VAR_DIR}"

exec docker run -p 8086:8086 -p 8089:8089 \
  -v "${VAR_DIR}:/var/lib/influxdb" \
  -e INFLUXDB_DB=test \
  -e INFLUXDB_HTTP_ENABLED=true \
  -e INFLUXDB_HTTP_FLUX_ENABLED=true \
  -e INFLUXDB_HTTP_BIND_ADDRESS=':8086' \
  -e INFLUXDB_UDP_ENABLED=true \
  -e INFLUXDB_UDP_BIND_ADDRESS=':8089' \
  -e INFLUXDB_UDP_DATABASE=test \
  -e INFLUXDB_REPORTING_DISABLED=true \
  influxdb:1.8-alpine
