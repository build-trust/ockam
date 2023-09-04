#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "relay - create relay with default parameters" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  fwd="$(random_str)"
  run_success "$OCKAM" relay create $fwd

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}
