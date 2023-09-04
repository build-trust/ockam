#!/bin/bash

# ===== SETUP

setup_file() {
  load load/base.bash
}

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

@test "message - send a message to a project node from an embedded node" {
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --to /project/default/service/echo
  assert_output "$msg"
}

@test "message - send a message to a project node from a background node" {
  run_success "$OCKAM" node create blue

  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --from /node/blue --to /project/default/service/echo
  assert_output "$msg"
}

@test "message - send a message to a project node from an embedded node, passing identity" {
  run_success "$OCKAM" identity create m1
  m1_identifier=$($OCKAM identity show m1)

  run_success "$OCKAM" project ticket --member "$m1_identifier" --attribute role=member

  # m1' identity was added by enroller
  run_success "$OCKAM" project enroll --identity m1

  # m1 is a member, must be able to contact the project' service
  msg=$(random_str)
  run_success "$OCKAM" message send --timeout 5 --identity m1 --to /project/default/service/echo "$msg"
  assert_output "$msg"

  # m2 is not a member, must not be able to contact the project' service
  run_success "$OCKAM" identity create m2
  run_failure "$OCKAM" message send --timeout 5 --identity m2 --to /project/default/service/echo "$msg"
}

@test "message - send a hex encoded message to a project node from an embedded node" {
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --to /project/default/service/echo --hex
  assert_output "$msg"
}
