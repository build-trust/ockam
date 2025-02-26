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

@test "portals - create an inlet/outlet pair, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$relay_name/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - inlet/outlet example with credential, not provided" {
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  # Setup nodes from a non-enrolled environment
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Green isn't enrolled as project member
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  relay_name="$(random_str)"
  run_success "$OCKAM" project-member add "$blue_identifier" --attribute role=member --relay $relay_name
  sleep 2

  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  assert_output --partial "forward_to_$relay_name"

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create --at /node/green --from $port --via $relay_name

  # Green can't establish secure channel with blue, because it didn't exchange credential with it.
  run_failure curl -sfI -m 3 "127.0.0.1:$port"
}
