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

@test "relay - create relay and send message through it" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # In two separate commands
  run_success $OCKAM relay create n2 --at /node/n1 --to /node/n2
  msg=$(random_str)
  run_success "$OCKAM" message send --timeout 5 "$msg" --to /node/n1/service/forward_to_n2/service/uppercase
  assert_output "$(to_uppercase "$msg")"

  # Piping the output of the first command into the second
  msg=$(random_str)
  run_success bash -c "$OCKAM relay create --at /node/n2 --to /node/n1 \
    | $OCKAM message send $msg --to /node/n2/-/service/uppercase"
  assert_output "$(to_uppercase "$msg")"
}

@test "relay - create two relays and list them on a node" {
  run_success --separate-stderr "$OCKAM" node create n1
  run_success --separate-stderr "$OCKAM" node create n2

  run_success $OCKAM relay create blue --at /node/n1 --to /node/n2
  run_success $OCKAM relay create red --at /node/n1 --to /node/n2

  run_success $OCKAM relay list --to /node/n2 --output json
  assert_output --partial "\"remote_address\": \"forward_to_blue\""
  assert_output --partial "\"remote_address\": \"forward_to_red\""

  # Test listing node with no relays
  run_success $OCKAM relay list --to /node/n1
  assert_output --partial "[]"
}

@test "relay - CRUD" {
  run_success --separate-stderr "$OCKAM" node create n1
  run_success --separate-stderr "$OCKAM" node create n2

  # Create and show it
  run_success "$OCKAM" relay create blue --at /node/n1 --to /node/n2
  run_success "$OCKAM" relay show forward_to_blue --at /node/n2 --output json
  assert_output --regexp "\"relay_route\".* => 0#forward_to_blue"
  assert_output --partial "\"remote_address\":\"/service/forward_to_blue\""
  assert_output --regexp "\"worker_address\":\"/service/.*"

  ## Try to show a non-existing relay
  run_failure "$OCKAM" relay show forward_to_r --at /node/n2
  assert_output --partial "not found"

  # Create another one and list both
  run_success "$OCKAM" relay create red --at /node/n1 --to /node/n2
  run_success "$OCKAM" relay list --to /node/n2 --output json
  assert_output --partial "\"remote_address\": \"forward_to_blue\""
  assert_output --partial "\"remote_address\": \"forward_to_red\""

  # Delete the first
  run_success "$OCKAM" relay delete -y forward_to_blue --at /node/n2
  run_success "$OCKAM" relay list --to /node/n2 --output json
  refute_output --partial "\"remote_address\": \"forward_to_blue\""
  assert_output --partial "\"remote_address\": \"forward_to_red\""

  ## Try to delete twice
  run_failure "$OCKAM" relay delete -y forward_to_blue --at /node/n2
}
