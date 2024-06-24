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
  ./run.sh cleanup || true
  popd
  teardown_home_dir
}

# pass
@test "examples - apis - nodejs amazon_ec2" {
  pushd examples/command/portals/apis/nodejs/amazon_ec2/aws_cli
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - apis - python amazon_ec2" {
  pushd examples/command/portals/apis/python/amazon_ec2/aws_cli
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}
