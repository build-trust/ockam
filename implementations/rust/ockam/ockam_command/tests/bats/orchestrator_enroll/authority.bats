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

@test "project authority - test api commands" {
  run "$OCKAM" identity create m
  m_identifier=$($OCKAM identity show m)

  run_success "$OCKAM" project-member list-ids
  run_success "$OCKAM" project-member list

  run_success "$OCKAM" project-member add "$m_identifier" --attribute key=value --relay="*"
  sleep 2

  run_success "$OCKAM" project-member list-ids
  assert_output --partial "$m_identifier"

  run_success "$OCKAM" project-member list

  assert_output --partial "$m_identifier"
  assert_output --partial "\"key\": \"value\""
  assert_output --partial "\"ockam-relay\": \"*\""

  run_success "$OCKAM" project-member delete "$m_identifier"
}

@test "project authority - test api authorization rules" {
  # Enroller
  run "$OCKAM" identity create e
  # Member
  run "$OCKAM" identity create m

  run "$OCKAM" identity create t

  e_identifier=$($OCKAM identity show e)
  m_identifier=$($OCKAM identity show m)
  t_identifier=$($OCKAM identity show t)

  run_success "$OCKAM" project-member add "$e_identifier" --enroller

  run_success "$OCKAM" project-member list-ids
  run_success "$OCKAM" project-member list

  # TODO: Should not work after we enable all checks on Authority nodes
  # run_failure "$OCKAM" project-member add "$m_identifier" --enroller --identity e
  run_success "$OCKAM" project-member add "$m_identifier" --identity e

  run_failure "$OCKAM" project-member list --identity m
  run_failure "$OCKAM" project-member list-ids --identity m
  run_failure "$OCKAM" project-member add "$t_identifier" --identity m
  run_failure "$OCKAM" project-member delete "$m_identifier" --identity m

  run_success "$OCKAM" project-member delete "$m_identifier" --identity e
}
