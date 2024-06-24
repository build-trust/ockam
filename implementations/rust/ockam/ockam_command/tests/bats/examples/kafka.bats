#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load ./setup.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  if [[ -z $BATS_TEST_COMPLETED ]]; then
    echo "Test failed $(pwd)" >&3
    cat log.txt >&3 || true
  fi

  ./run.sh cleanup $EXTRA_ARG || true
  unset EXTRA_ARG
  cd -
  teardown_home_dir
}

# ===== TESTS

@test "examples - kafka - aiven serverless" {
  skip
  container_to_watch="application_team-consumer"
  cd examples/command/portals/kafka/aiven
  ./run.sh >/dev/null &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - kafka - apache docker" {
  container_to_watch="application_team-consumer"

  cd examples/command/portals/kafka/apache/docker
  ./run.sh >log.txt &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - kafka - confluent serverless" {
  skip
  container_to_watch="application_team-consumer"
  cd examples/command/portals/kafka/confluent
  ./run.sh >/dev/null &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - kafka - instaclustr serverless" {
  skip
  container_to_watch="application_team-consumer"
  cd examples/command/portals/kafka/instaclustr/docker
  ./run.sh >/dev/null &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch" "900s"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - kafka - redpanda docker" {
  container_to_watch="application_team-consumer"

  cd examples/command/portals/kafka/redpanda/docker
  ./run.sh >log.txt &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - kafka - warpstream serverless" {
  export EXTRA_ARG="$WARPSTREAM_API_KEY"
  container_to_watch="application_team-consumer"

  cd examples/command/portals/kafka/warpstream
  ./run.sh $WARPSTREAM_API_KEY >/dev/null &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}
