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

@test "message - send a message to a project node from a background node" {
  run_success "$OCKAM" node create blue

  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --from /node/blue --to /project/default/service/echo
  assert_output "$msg"
}

@test "message - send a hex encoded message to a project node from an embedded node" {
  msg=$(random_hex_str)
  run_success "$OCKAM" message send "$msg" --to /project/default/service/echo --hex
  assert_output "$msg"
}
