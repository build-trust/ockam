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

@test "identity - export/import" {
  # Export the enrolled identity
  run_success "$OCKAM" identity export
  exported=$output

  # Import it in a new ockam home and use it
  setup_home_dir
  run_failure "$OCKAM" zone list
  assert_output --partial "401" # unauthorized error
  run_success "$OCKAM" reset --yes
  run_success "$OCKAM" identity import "$exported"
  run_success "$OCKAM" zone list

  # Check it's enrolled
  email=$($OCKAM status --output json | jq -r ".identities[] | select(.status.Enrolled != null) | .status.Enrolled.email")
  [ "$email" != "null" ] || {
    echo "Email is null"
    exit 1
  }
}
