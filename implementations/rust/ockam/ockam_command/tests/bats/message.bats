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
  run "$OCKAM" message send "$msg" --to /project/default/service/echo
  assert_success
  assert_output "$msg"
}

@test "message - send a message to a project node from a background node" {
  run "$OCKAM" node create blue
  assert_success

  msg=$(random_str)
  run "$OCKAM" message send "$msg" --from /node/blue --to /project/default/service/echo
  assert_success
  assert_output "$msg"
}

@test "message - send a message to a project node from an embedded node, passing identity" {
  run "$OCKAM" identity create m1
  assert_success
  m1_identifier=$($OCKAM identity show m1)

  run "$OCKAM" project ticket --member "$m1_identifier" --attribute role=member
  assert_success

  # m1' identity was added by enroller
  run "$OCKAM" project enroll --identity m1
  assert_success

  # m1 is a member, must be able to contact the project' service
  msg=$(random_str)
  run "$OCKAM" message send --timeout 5 --identity m1 --to /project/default/service/echo "$msg"
  assert_success
  assert_output "$msg"

  # m2 is not a member, must not be able to contact the project' service
  run "$OCKAM" identity create m2
  assert_success
  run "$OCKAM" message send --timeout 5 --identity m2 --to /project/default/service/echo "$msg"
  assert_failure
}
