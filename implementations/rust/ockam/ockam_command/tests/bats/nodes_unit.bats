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

@test "node - can recreate a background node after it was stopped" {
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

@test "node - can recreate a foreground node after it was stopped" {
  n="$(random_str)"
  $OCKAM node create $n -f &
  sleep 0.1
  run "$OCKAM" node show "$n"
  assert_success

  # Fail to create a node with the same name
  run "$OCKAM" node create "$n" -f
  assert_failure

  run "$OCKAM" node stop "$n"
  assert_success

  # Recreate node
  $OCKAM node create $n -f &
  sleep 0.1
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
  sleep 0.1

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
  sleep 0.1

  log_file="$($OCKAM node logs $n)"
  if [ -s $log_file ]; then
    fail "Log file should be empty"
  fi
}
