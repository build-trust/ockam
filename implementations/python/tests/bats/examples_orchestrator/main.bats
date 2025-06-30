#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/base_python.bash
  load ../load/orchestrator.bash
  load ../load/orchestrator_python.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
  teardown_zone_test
}

# ===== TESTS

# For most of the examples, we run `$OCKAM --rm` in a subshell, store its PID, and keep trying
# for 5 minutes a `curl http://localhost:$DEFAULT_HTTP_PORT/agents` until it returns a response that
# contains `.agents[0].name == "henry"`. In others, we wait until the public endpoint is reachable.

get_agents_response() {
  # Wait for the Ockam Orchestrator to start and return the agents
  curl -sf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "localhost:$DEFAULT_HTTP_PORT/agents"
  # Sometimes it takes a bit to return the populated json after the endpoint is reachable.
  # To prevent flakyness, we wait a couple of seconds and run a curl without retrying to get the final response.
  sleep 2
  run_success curl -sf -m 5 "localhost:$DEFAULT_HTTP_PORT/agents"
}

@test "example 001" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/001
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 002" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/002
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 003" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/003
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 004" {
  example_dir="$MAIN_EXAMPLES_DIR"/004
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example004')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}

@test "example 005" {
  example_dir="$MAIN_EXAMPLES_DIR"/005
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  pushd data >/dev/null || return 1
  python3 -m http.server --bind 127.0.0.1 5555 &
  add_background_pid $!
  popd >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  sleep 30 # wait until the zone is created so we can generate an enrollment ticket
  $OCKAM zone outlet --relay files --to 127.0.0.1:5555 &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example005')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}

@test "example 006" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/006
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 007" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/007
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 008" {
  wait_for_closed_port "$DEFAULT_HTTP_PORT" 30
  example_dir="$MAIN_EXAMPLES_DIR"/008
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  get_agents_response
  res="$(echo $output | jq -r '.agents[0].name')"
  assert_equal "$res" "henry"
}

@test "example 009" {
  example_dir="$MAIN_EXAMPLES_DIR"/009
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example009')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}

@test "example 010" {
  example_dir="$MAIN_EXAMPLES_DIR"/010
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example010')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}

@test "example 011" {
  example_dir="$MAIN_EXAMPLES_DIR"/011
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example011')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}

@test "example 012" {
  example_dir="$MAIN_EXAMPLES_DIR"/012
  check_dir_exists "$example_dir"
  pushd "$example_dir" >/dev/null || return 1

  $OCKAM --rm --no-logs &
  add_background_pid $!

  # validate the public endpoint is reachable and returns an ok response
  endpoint="$(public_endpoint 'example012')"
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 60 -m 5 "$endpoint"
}
