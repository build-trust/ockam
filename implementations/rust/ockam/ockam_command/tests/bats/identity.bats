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

@test "identity - create and check show output" {
  i=$(random_str)
  run_success "$OCKAM" identity create "${i}"
  run_success "$OCKAM" identity show "${i}"
  assert_output --regexp '^I'

  run_success "$OCKAM" identity show "${i}" --full
  assert_output --partial "Change[0]:"
  assert_output --partial "Identifier: "
  assert_output --partial "primary_public_key: "
}

@test "identity - CRUD" {
  # Create with random name
  run_success "$OCKAM" identity create

  # Create a named identity and delete it
  i=$(random_str)
  run_success "$OCKAM" identity create "${i}"
  run_success "$OCKAM" identity delete "${i}" --yes

  # Fail to delete identity when it's in use by a node
  i=$(random_str)
  n=$(random_str)

  run_success "$OCKAM" identity create "${i}"
  run_success "$OCKAM" node create "${n}" --identity "${i}"
  run_failure "$OCKAM" identity delete "${i}" --yes

  # Delete identity after deleting the node
  run_success "$OCKAM" node delete "${n}" --yes
  run_success "$OCKAM" identity delete "${i}" --yes
}

@test "identity - set default" {
  i=$(random_str)

  run_success "$OCKAM" identity create "${i}"

  run_success "$OCKAM" identity default
  assert_output --partial "The name of the default identity is '${i}'"

  run_failure "$OCKAM" identity default "${i}"
  assert_output --partial "The identity named '${i}' is already the default"

  i=$(random_str)
  run_success "$OCKAM" identity create "${i}"
  run_success "$OCKAM" identity default "${i}"
  assert_output "${i}"
}
