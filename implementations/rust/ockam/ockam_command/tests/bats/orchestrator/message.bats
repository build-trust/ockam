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

@test "message - send a message to a project node from an embedded node" {
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --to /project/default/service/echo
  assert_output "$msg"
}
