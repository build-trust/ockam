#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "relay - create relay with default parameters" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create $relay_name

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$relay_name/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}
