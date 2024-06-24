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

  ./run.sh cleanup || true
  popd || true
  teardown_home_dir
}

# ===== TESTS

# fail - bad
# @test "examples - database - influxdb amazon_timestream" {
#   cd examples/command/portals/databases/influxdb/amazon_timestream/aws_cli
#   run_success ./run.sh
#   assert_output --partial "The example run was successful 🥳."$'\n'
# }

@test "examples - database - mongodb amazon_vpc" {
  pushd examples/command/portals/databases/mongodb/amazon_vpc
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - database - mongodb docker" {
  pushd examples/command/portals/databases/mongodb/docker
  ./run.sh >log.txt &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  container_to_watch="analysis_corp-app"
  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - database - mongodb kubernetes" {
  pushd examples/command/portals/databases/mongodb/kubernetes
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - database - postgres amazon_aurora" {
  pushd examples/command/portals/databases/postgres/amazon_aurora/aws_cli
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - database - postgres amazon_rds" {
  pushd examples/command/portals/databases/postgres/amazon_rds/aws_cli
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - database - postgres docker" {
  pushd examples/command/portals/databases/postgres/docker
  ./run.sh >log.txt &
  BGPID=$!
  trap 'kill $BGPID; exit' INT

  container_to_watch="analysis_corp-app"
  run_success wait_till_container_starts "$container_to_watch"

  exit_on_successful "$container_to_watch" &

  wait_till_successful_run_or_error "$container_to_watch"
  assert_equal "$exit_code" "0"
}

@test "examples - database - postgres kubernetes" {
  pushd examples/command/portals/databases/postgres/kubernetes
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}
