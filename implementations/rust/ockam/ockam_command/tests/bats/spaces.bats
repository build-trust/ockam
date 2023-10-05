#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "spaces - fail with human readable error if not enrolled" {
  run_failure "$OCKAM" space create
  assert_output --partial "Please enroll using 'ockam enroll' before using this command"
}
