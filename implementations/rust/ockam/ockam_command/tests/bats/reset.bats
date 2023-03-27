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
  run mkdir -p "$OCKAM_HOME"
  run touch "$OCKAM_HOME"/env
  assert_success

  assert_success
  run touch "$OCKAM_HOME"/bin/ockam

  # create a ockam bin directory and a fake ockam executable in the OCKAM_HOME directory
  run mkdir "$OCKAM_HOME"/bin
  assert_success

  # create some state in the OCKAM_HOME directory
  run "$OCKAM" node create
  assert_success

  run ls "$OCKAM_HOME"
  assert_output --partial "nodes"

  # reset the OCKAM_HOME directory
  run "$OCKAM" reset --yes

  # list all remaining files and directories
  run ls "$OCKAM_HOME"
  assert_output 'bin
env'

  # reset the OCKAM_HOME directory twice, this should not fail
  run "$OCKAM" reset --yes
  run ls "$OCKAM_HOME"
  assert_output 'bin
env'
}
