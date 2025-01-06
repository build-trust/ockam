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

@test "relay - create relay with default parameters" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create $relay_name

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$relay_name/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "relay - control who can create/claim a relay" {
  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  # Green isn't enrolled as project member
  relay_name_blue="$(random_str)"
  relay_name_green="$(random_str)"
  run_success "$OCKAM" project-member add "$blue_identifier" --attribute role=member --relay $relay_name_blue
  run_success "$OCKAM" project-member add "$green_identifier" --attribute role=member --relay $relay_name_green
  sleep 2

  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Blue can take its relay
  run_success "$OCKAM" relay create $relay_name_blue --to /node/blue
  # Green can't take blue's relay
  run_success "$OCKAM" relay create $relay_name_blue --to /node/green --jq ".connection_status"
  assert_output "\"Down\""
  # But can take its the one it was assigned to in the ticket
  run_success "$OCKAM" relay create $relay_name_green --to /node/green

  run_success "$OCKAM" node create admin_node

  # Admin can take any relay (has wildcard *)
  run_success "$OCKAM" relay create $relay_name_blue --to /node/admin_node
  run_success "$OCKAM" relay create $relay_name_green --to /node/admin_node
}
