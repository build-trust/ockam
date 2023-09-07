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
  run_success "$OCKAM" service start identity --addr my_identity --at n1
  run_failure "$OCKAM" service start identity --addr my_identity --at n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start authenticated --addr my_authenticated --at n1
  run_failure "$OCKAM" service start authenticated --addr my_authenticated --at n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start verifier --addr my_verifier --at n1
  run_failure "$OCKAM" service start verifier --addr my_verifier --at n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start credentials --addr my_credentials --at n1 --identity 0134dabe4f886af3bd5d2b3ab50891a6dfe90c99099668ce8cb680888cac7d67db000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e1acf2670f5bfc34c466910949618c68a53183976e8e57d5fc07b6a3d02d22a3030101407e6332d0deeccf8d12de9972e31b54200f1597db2a195d08b15b251d6293c180611c66acc26913a16d5ea5536227c8baefb4fa95bd709212fdc1ca4fc3370e02
  run_failure "$OCKAM" service start credentials --addr my_credentials --at n1 --identity 0134dabe4f886af3bd5d2b3ab50891a6dfe90c99099668ce8cb680888cac7d67db000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e1acf2670f5bfc34c466910949618c68a53183976e8e57d5fc07b6a3d02d22a3030101407e6332d0deeccf8d12de9972e31b54200f1597db2a195d08b15b251d6293c180611c66acc26913a16d5ea5536227c8baefb4fa95bd709212fdc1ca4fc3370e02
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
