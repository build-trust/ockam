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

@test "reset must keep only the env and bin directories" {
  # create a fake env file in the OCKAM_HOME directory
  run_success mkdir -p "$OCKAM_HOME"
  run_success touch "$OCKAM_HOME"/env

  # create a ockam bin directory and a fake ockam executable in the OCKAM_HOME directory
  run_success mkdir "$OCKAM_HOME"/bin
  run_success touch "$OCKAM_HOME"/bin/ockam

  # create some state in the OCKAM_HOME directory
  run_success "$OCKAM" node create
  run_success ls "$OCKAM_HOME"
  assert_output --partial "nodes"

  # reset the OCKAM_HOME directory
  run_success "$OCKAM" reset --yes

  # list all remaining files and directories
  run_success ls "$OCKAM_HOME"
  assert_output 'bin
env'

  # reset the OCKAM_HOME directory twice, this should not fail
  run_success "$OCKAM" reset --yes
  run_success ls "$OCKAM_HOME"
  assert_output 'bin
env'
}
