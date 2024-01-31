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

# ===== UTILS

force_kill_node() {
  max_retries=5
  i=0
  while [[ $i -lt $max_retries ]]; do
    pid="$($OCKAM node show $1 --output json | jq .node_pid)"
    run kill -9 $pid
    # Killing a node created without `-f` leaves the
    # process in a defunct state when running within Docker.
    if ! ps -p $pid || ps -p $pid | grep defunct; then
      return
    fi
    sleep 0.2
    ((i = i + 1))
  done
}

# ===== TESTS

@test "node - create with random name" {
  run_success "$OCKAM" node create
}

@test "node - create with name" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  run_success "$OCKAM" node show "$n"
  assert_output --partial "/dnsaddr/localhost/tcp/"
  assert_output --partial "/service/api"
  assert_output --partial "/service/uppercase"
}

@test "node - start services" {
  run_success "$OCKAM" node create n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start hop --addr my_hop --at n1
  run_failure "$OCKAM" service start hop --addr my_hop --at n1
}

@test "node - is restarted with default services" {
  n="$(random_str)"
  # Create node, check that it has one of the default services running
  run_success "$OCKAM" node create "$n"
  assert_output --partial "Node ${n} created successfully"

  # Stop node, restart it, and check that the service is up again
  $OCKAM node stop "$n"
  run_success "$OCKAM" node start "$n"
  assert_output --partial "/service/echo"
}

@test "node - fail to create two background nodes with the same name" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"
  run_failure "$OCKAM" node create "$n"
}

@test "node - can recreate a background node after it was gracefully stopped" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"
  run_success "$OCKAM" node stop "$n"
  # Recreate node
  run_success "$OCKAM" node create "$n"
}

@test "node - can recreate a background node after it was killed" {
  # This test emulates the situation where a node is killed by the OS
  # on a restart or a shutdown. The node should be able to restart without errors.
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  force_kill_node "$n"

  # Recreate node
  run_success "$OCKAM" node create "$n"
}

@test "node - fail to create two foreground nodes with the same name" {
  n="$(random_str)"
  run_success "$OCKAM" node create $n -f &
  sleep 1
  run_success "$OCKAM" node show "$n"
  run_failure "$OCKAM" node create "$n" -f
}

@test "node - can recreate a foreground node after it was killed" {
  n="$(random_str)"
  run_success "$OCKAM" node create $n -f &
  sleep 1
  run_success "$OCKAM" node show "$n"

  force_kill_node "$n"

  # Recreate node
  run_success "$OCKAM" node create $n -f &
  sleep 1
  run_success "$OCKAM" node show "$n"
}

@test "node - can recreate a foreground node after it was gracefully stopped" {
  n="$(random_str)"
  run_success "$OCKAM" node create $n -f &
  sleep 1
  run_success "$OCKAM" node show "$n"

  run_success "$OCKAM" node stop "$n"

  # Recreate node
  run_success "$OCKAM" node create $n -f &
  sleep 1
  run_success "$OCKAM" node show "$n"
}

@test "node - background node logs to file" {
  QUIET=0
  n="$(random_str)"
  run_success "$OCKAM" node create $n
  run_success ls -l "$OCKAM_HOME/nodes/$n"
  assert_output --partial "stdout"
}

@test "node - foreground node logs to stdout only" {
  n="$(random_str)"
  run_success "$OCKAM" node create $n -vv -f &
  sleep 1
  # It should even create the node directory
  run_failure ls -l "$OCKAM_HOME/nodes/$n"
}
