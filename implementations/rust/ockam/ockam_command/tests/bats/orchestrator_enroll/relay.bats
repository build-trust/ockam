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

@test "relay - create relay between rust nodes requires credentials" {
  relay_name=$(random_str)
  relay_ticket_path="$OCKAM_HOME/relay.ticket"

  run_success bash -c "$OCKAM project ticket --usage-count 1 --relay $relay_name > $relay_ticket_path"

  setup_home_dir
  $OCKAM project enroll $relay_ticket_path

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" node create red

  # fail with a different relay name
  run_failure "$OCKAM" relay create --at /node/blue/secure/api --to red unauthorized_relay_name
  run_success "$OCKAM" relay create --at /node/blue/secure/api --to red $relay_name
}
