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

# ===== TESTS

@test "examples - ai - amazon_bedrock" {
  pushd examples/command/portals/ai/amazon_bedrock
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}

@test "examples - ai - amazon_ec2" {
  pushd examples/command/portals/ai/amazon_ec2
  run_success ./run.sh
  assert_output --partial "The example run was successful 🥳."$'\n'
}
