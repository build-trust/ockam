#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

# https://docs.ockam.io/guides/use-cases/add-end-to-end-encryption-to-any-client-and-server-application-with-no-code-change
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end encryption, local" {
  port="$(random_port)"
  run_success "$OCKAM" node create relay

  # Service
  run_success "$OCKAM" node create server_sidecar

  run_success "$OCKAM" tcp-outlet create --at /node/server_sidecar --to "$PYTHON_SERVER_PORT"
  run_success "$OCKAM" relay create server_sidecar --at /node/relay --to /node/server_sidecar
  assert_output --partial "forward_to_server_sidecar"

  # Client
  run_success "$OCKAM" node create client_sidecar
  run_success bash -c "$OCKAM secure-channel create --from /node/client_sidecar --to /node/relay/service/forward_to_server_sidecar/service/api \
              | $OCKAM tcp-inlet create --at /node/client_sidecar --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}
