#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "identity - create and check show output" {
  run_success "$OCKAM" identity create i
  run_success "$OCKAM" identity show i
  assert_output --regexp '^I'

  run_success "$OCKAM" identity show i --full
  assert_output --partial "Change[0]:"
  assert_output --partial "Identifier: "
  assert_output --partial "primary_public_key: "
}

@test "identity - CRUD" {
  # Create with random name
  run_success "$OCKAM" identity create

  # Create a named identity and delete it
  run_success "$OCKAM" identity create i
  run_success "$OCKAM" identity delete i --yes

  # Fail to delete identity when it's in use by a node
  run_success "$OCKAM" identity create i
  run_success "$OCKAM" node create n --identity i
  run_failure "$OCKAM" identity delete i --yes

  # Delete identity after deleting the node
  run_success "$OCKAM" node delete n --yes
  run_success "$OCKAM" identity delete i --yes

  # Create two and list them
  run_success "$OCKAM" identity create i1
  run_success "$OCKAM" identity create i2
  run_success "$OCKAM" identity list
  assert_output --partial i1
  assert_output --partial i2

  # Update the list correctly after deleting one
  run_success "$OCKAM" identity delete i1 --yes
  run_success "$OCKAM" identity list
  assert_output --partial i2
  refute_output --partial i1

  # Delete twice
  run_failure "$OCKAM" identity delete i1 --yes

  # Delete all and check that the list is empty
  run_success "$OCKAM" identity delete --all --yes

  run_success "$OCKAM" identity list --output json
  assert_output --partial "[]"

  # Delete on empty list
  run_success "$OCKAM" identity delete
  assert_output --partial "[]"
}

@test "identity - set default" {
  run_success "$OCKAM" identity create i1

  run_success "$OCKAM" identity default
  assert_output --partial "The name of the default identity is 'i1'"

  run_failure "$OCKAM" identity default i1
  assert_output --partial "The identity named 'i1' is already the default"

  run_success "$OCKAM" identity create i2
  run_success "$OCKAM" identity default i2
  assert_output i2
}

@test "identity - export/import" {
  # Create and export
  run_success "$OCKAM" identity create
  run_success "$OCKAM" identity show --full --encoding hex
  exported=$output

  # Remove it
  run_success "$OCKAM" identity delete --all --yes
  run_success "$OCKAM" identity list --output json
  assert_output --partial "[]"

  # Import it back
  run_success "$OCKAM" identity create --identity "$exported"
  run_success "$OCKAM" identity show --full --encoding hex
  assert_output "$exported"
}
