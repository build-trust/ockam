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
    pid="$(cat $OCKAM_HOME/nodes/$1/pid)"
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
  assert_success

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start authenticated --addr my_authenticated --at n1
  run_failure "$OCKAM" service start authenticated --addr my_authenticated --at n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start credentials --addr my_credentials --at n1 --identity 81a201583ba20101025835a4028201815820984249b1a11c6933002d02019f408ec0bdb7f3058068227a472986ea588ec67003f4041a64e49a5e051a77b09d5e02820181584002c2cc20acf3d7d59d67c420c3c29d4ebb1ebe483bfaba7fb046f59de96284ebfb570d17539e5d4989b74f22af12261b9c1d5eecf731e2d19907b092f6c47d04
  run_failure "$OCKAM" service start credentials --addr my_credentials --at n1 --identity 81a201583ba20101025835a4028201815820984249b1a11c6933002d02019f408ec0bdb7f3058068227a472986ea588ec67003f4041a64e49a5e051a77b09d5e02820181584002c2cc20acf3d7d59d67c420c3c29d4ebb1ebe483bfaba7fb046f59de96284ebfb570d17539e5d4989b74f22af12261b9c1d5eecf731e2d19907b092f6c47d04
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

  log_file="$($OCKAM node logs $n)"
  if [ ! -s $log_file ]; then
    fail "Log file shouldn't be empty"
  fi
}

@test "node - foreground node logs to stdout only" {
  n="$(random_str)"
  run_success "$OCKAM" node create $n -vv -f &
  sleep 1

  log_file="$($OCKAM node logs $n)"
  if [ -s $log_file ]; then
    fail "Log file should be empty"
  fi
}
