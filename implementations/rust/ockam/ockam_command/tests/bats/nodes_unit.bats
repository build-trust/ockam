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
  pid="$(cat $OCKAM_HOME/nodes/$1/pid)"
  max_retries=5
  i=0
  while [[ $i -lt $max_retries ]]; do
    run kill -9 $pid
    if [ $status -ne 0 ]; then
      break
    fi
    sleep 0.2
    ((i=i+1))
  done
  if ps -p $pid >/dev/null; then
    fail "Failed to kill node $1"
  fi
}

# ===== TESTS

@test "node - can recreate a background node after it was gracefully stopped" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  # Fail to create a node with the same name
  run "$OCKAM" node create "$n"
  assert_failure

  run "$OCKAM" node stop "$n"
  assert_success

  # Recreate node
  run "$OCKAM" node create "$n"
  assert_success
}

@test "node - can recreate a background node after it was killed" {
  # TODO: move to rust tests
  skip "The 'kill' command doesn't work as expected on CI. This test will be moved to rust tests soon."
  # This test emulates the situation where a node is killed by the OS
  # on a restart or a shutdown. The node should be able to restart without errors.
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  # Fail to create a node with the same name
  run "$OCKAM" node create "$n"
  assert_failure

  force_kill_node "$n"
  force_kill_node "$n"

  # Recreate node
  run "$OCKAM" node create "$n"
  assert_success
}

@test "node - can recreate a foreground node after it was killed" {
  # TODO: move to rust tests
  skip "The 'kill' command doesn't work as expected on CI. This test will be moved to rust tests soon."
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 0.2
  run "$OCKAM" node show "$n"
  assert_success

  # Fail to create a node with the same name
  run "$OCKAM" node create "$n" -f
  assert_failure

  force_kill_node "$n"

  # Recreate node
  $OCKAM node create $n -f &
  sleep 0.2
  run "$OCKAM" node show "$n"
  assert_success
}

@test "node - can recreate a foreground node after it was gracefully stopped" {
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 0.2
  run "$OCKAM" node show "$n"
  assert_success

  # Fail to create a node with the same name
  run "$OCKAM" node create "$n" -f
  assert_failure

  run "$OCKAM" node stop "$n"
  assert_success

  # Recreate node
  $OCKAM node create $n -f &
  sleep 0.2
  run "$OCKAM" node show "$n"
  assert_success
}

@test "node - logs to file" {
  n="$(random_str)"
  $OCKAM node create $n

  log_file="$($OCKAM node logs $n)"
  if [ ! -s $log_file ]; then
    fail "Log file shouldn't be empty"
  fi

  # Repeat the same with a foreground node
  n="$(random_str)"
  $OCKAM node create $n -vv -f &
  sleep 0.2

  log_file="$($OCKAM node logs $n)"
  if [ ! -s $log_file ]; then
    fail "Log file shouldn't be empty"
  fi
}

@test "node - disable file logging" {
  n="$(random_str)"
  $OCKAM node create $n --disable-file-logging

  log_file="$($OCKAM node logs $n)"
  if [ -s $log_file ]; then
    fail "Log file should be empty"
  fi

  # Repeat the same with a foreground node
  n="$(random_str)"
  $OCKAM node create $n -vv -f --disable-file-logging &
  sleep 0.2

  log_file="$($OCKAM node logs $n)"
  if [ -s $log_file ]; then
    fail "Log file should be empty"
  fi
}
