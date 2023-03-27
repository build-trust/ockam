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
  load_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "spaces - list" {
  run "$OCKAM" space list
  assert_success
}
