#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "tcp connection - CRUD" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1 --tcp-listener-address "127.0.0.1:$port"

  # Create tcp-connection and check output
  addr=$("$OCKAM" tcp-connection create --from n1 --to "127.0.0.1:$port")

  # Check that the connection is listed
  run_success "$OCKAM" tcp-connection list --at n1 --output json
  assert_output --partial "$addr"
  assert_output --partial "127.0.0.1:$port"

  # Show the connection details
  run_success "$OCKAM" tcp-connection show --at n1 "$addr" --output json
  assert_output --partial "$addr"
  assert_output --partial "127.0.0.1:$port"

  # It also works passing the socket address
  run_success "$OCKAM" tcp-connection show --at n1 "127.0.0.1:$port" --output json
  assert_output --partial "$addr"
  assert_output --partial "127.0.0.1:$port"

  # Delete the connection
  run_success "$OCKAM" tcp-connection delete --at n1 "$addr" --yes

  # Check that it's no longer listed
  run_success "$OCKAM" tcp-connection list --at n1 --output json
  refute_output --partial "$addr"
  refute_output --partial "127.0.0.1:$port"
}

@test "tcp listener - CRUD" {
  run_success "$OCKAM" node create n1

  # Create tcp-listener and check output
  port="$(random_port)"
  addr="127.0.0.1:$port"
  run_success "$OCKAM" tcp-listener create "$addr" --at n1
  assert_output --regexp '/dnsaddr/localhost/tcp/[[:digit:]]+'

  # Check that the listener is listed
  run_success "$OCKAM" tcp-listener list --at n1
  assert_output --partial "$addr"

  # Show the listener details
  run_success "$OCKAM" tcp-listener show --at n1 "$addr"
  assert_output --partial "$addr"

  # Delete the listener
  run_success "$OCKAM" tcp-listener delete --at n1 "$addr" --yes

  # Check that it's no longer listed
  run_success "$OCKAM" tcp-listener list --at n1
  refute_output --partial "$addr"
}

@test "tcp - create a tcp connection and then delete it" {
  port="$(random_port)"
  addr="127.0.0.1:$port"
  run_success "$OCKAM" node create n1 --tcp-listener-address "$addr"
  run_success "$OCKAM" tcp-connection create --from n1 --to "$addr" --output json
}
