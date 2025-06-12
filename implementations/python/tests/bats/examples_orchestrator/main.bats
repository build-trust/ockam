#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/base_python.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
  kill_ockam_pid
}

# ===== TESTS

# For most of the examples, we run `$OCKAM --rm` in a subshell, store its PID, and keep trying
# for 5 minutes a `curl http://localhost:8000/agents` until it returns a response that
# contains `.agents[0].name == "henry"`

@test "example 001" {
  example_dir="$MAIN_EXAMPLES_DIR"/001
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 002" {
  example_dir="$MAIN_EXAMPLES_DIR"/002
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 003" {
  example_dir="$MAIN_EXAMPLES_DIR"/003
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 004" {
  example_dir="$MAIN_EXAMPLES_DIR"/004
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 005" {
  skip "Failing to connect to localhost:5555"

  example_dir="$MAIN_EXAMPLES_DIR"/005
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 006" {
  example_dir="$MAIN_EXAMPLES_DIR"/006
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 007" {
  example_dir="$MAIN_EXAMPLES_DIR"/007
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 008" {
  example_dir="$MAIN_EXAMPLES_DIR"/008
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000/agents"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 009" {
  example_dir="$MAIN_EXAMPLES_DIR"/009
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000"
  res="$(echo $output | jq -r '.agents | length')"
  assert_equal "$res" "5"
}

@test "example 010" {
  example_dir="$MAIN_EXAMPLES_DIR"/010
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  run_success curl -sf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "localhost:8000"
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 011" {
  example_dir="$MAIN_EXAMPLES_DIR"/011
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  # validate the public endpoint is reachable and returns an ok response
  public_endpoint="$(public_endpoint 'example-011')"
  run_success curl -sSf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "$public_endpoint"
}

@test "example 012" {
  example_dir="$MAIN_EXAMPLES_DIR"/012
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  export OCKAM_PID=$!

  # validate the public endpoint is reachable and returns an ok response
  public_endpoint="$(public_endpoint 'example-012')"
  run_success curl -sSf --retry-all-errors --retry-delay 30 --retry 10 -m 5 "$public_endpoint"
}
