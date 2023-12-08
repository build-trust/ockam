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

@test "relay - create relay with default parameters" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  fwd="$(random_str)"
  run_success "$OCKAM" relay create $fwd

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "relay - control who can create/claim a relay" {
  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  # Green isn't enrolled as project member
  fwd_blue="$(random_str)"
  fwd_green="$(random_str)"
  run_success "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member --relay $fwd_blue
  run_success "$OCKAM" project ticket --member "$green_identifier" --attribute role=member --relay $fwd_green

  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Blue can take its relay
  run_success "$OCKAM" relay create $fwd_blue --to /node/blue
  # Green can't take blue's relay
  run_failure "$OCKAM" relay create $fwd_blue --to /node/green
  # But can take its the one it was assigned to in the ticket
  run_success "$OCKAM" relay create $fwd_green --to /node/green

  run_success "$OCKAM" node create admin_node

  # Admin can take any relay (has wildcard *)
  run_success "$OCKAM" relay create $fwd_blue --to /node/admin_node
  run_success "$OCKAM" relay create $fwd_green --to /node/admin_node
}
