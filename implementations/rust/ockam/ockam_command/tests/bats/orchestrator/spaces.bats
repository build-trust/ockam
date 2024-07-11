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

@test "spaces - list" {
  run_success "$OCKAM" space list
}

@test "spaces - CRUD admins" {
  # get space admin email (the one used to enroll)
  run_success "$OCKAM" space-admin list --jq ".[0].email"
  assert_output --partial "@"
  enrolled_email=$output

  run_success "$OCKAM" space-admin add ockam.admin.test@ockam.io
  assert_output --partial "ockam.admin.test@ockam.io"

  run_failure "$OCKAM" space-admin add "not_an_email"

  run_success "$OCKAM" space-admin list --output json
  assert_output --partial "\"email\":\"ockam.admin.test@ockam.io\""

  # can't delete the admin with the same email as the enroller
  run_failure "$OCKAM" space-admin delete $enrolled_email --yes
  # can delete the added admin
  run_success "$OCKAM" space-admin delete ockam.admin.test@ockam.io --yes

  run_success "$OCKAM" space-admin list --jq ".[].email"
  assert_output --partial $enrolled_email
}
