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

@test "node - fail to create two background nodes with the same name" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  run "$OCKAM" node create "$n"
  assert_failure
}

@test "node - can recreate a background node after it was gracefully stopped" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  run "$OCKAM" node stop "$n"
  assert_success

  # Recreate node
  run "$OCKAM" node create "$n"
  assert_success
}

@test "node - can recreate a background node after it was killed" {
  # This test emulates the situation where a node is killed by the OS
  # on a restart or a shutdown. The node should be able to restart without errors.
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  force_kill_node "$n"

  # Recreate node
  run "$OCKAM" node create "$n"
  assert_success
}

@test "node - fail to create two foreground nodes with the same name" {
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 1
  run "$OCKAM" node show "$n"
  assert_success

  run "$OCKAM" node create "$n" -f
  assert_failure
}

@test "node - can recreate a foreground node after it was killed" {
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 1
  run "$OCKAM" node show "$n"
  assert_success

  force_kill_node "$n"

  # Recreate node
  $OCKAM node create $n -f &
  sleep 1
  run "$OCKAM" node show "$n"
  assert_success
}

@test "node - can recreate a foreground node after it was gracefully stopped" {
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 1
  run "$OCKAM" node show "$n"
  assert_success

  run "$OCKAM" node stop "$n"
  assert_success

  # Recreate node
  $OCKAM node create $n -f &
  sleep 1
  run "$OCKAM" node show "$n"
  assert_success
}

@test "node - background node logs to file" {
  n="$(random_str)"
  $OCKAM node create $n

  log_file="$($OCKAM node logs $n)"
  if [ ! -s $log_file ]; then
    fail "Log file shouldn't be empty"
  fi
}

@test "node - foreground node logs to stdout only" {
  n="$(random_str)"
  $OCKAM node create $n -vv -f &
  sleep 1

  log_file="$($OCKAM node logs $n)"
  if [ -s $log_file ]; then
    fail "Log file should be empty"
  fi
}
