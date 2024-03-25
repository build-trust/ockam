#!/bin/bash

# ===== SETUP

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
@test "project authority - test api commands" {
  run "$OCKAM" identity create m
  m_identifier=$($OCKAM identity show m)

  run_success "$OCKAM" project-member list-ids
  run_success "$OCKAM" project-member list

  run_success "$OCKAM" project-member add "$m_identifier" --attribute key=value --relay="*"

  run_success "$OCKAM" project-member list-ids
  assert_output --partial "$m_identifier"

  run_success "$OCKAM" project-member list

  assert_output --partial "$m_identifier"
  assert_output --partial "key=value"
  assert_output --partial "ockam-relay=*"

  run_success "$OCKAM" project-member delete "$m_identifier"
}
