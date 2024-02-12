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

@test "policies - create resource type policy, backwards compatibility" {
  run_success $OCKAM policy create --resource tcp-outlet --expression '(= subject.component "global_value")'
  run_success $OCKAM policy show --resource-type tcp-outlet
  assert_output --partial "tcp-outlet"
  assert_output --partial "(= subject.component \"global_value\")"

  run_success $OCKAM policy delete --resource-type tcp-outlet -y
  run_success $OCKAM policy show --resource-type tcp-outlet
  refute_output --partial "tcp-outlet"
}

@test "policies - create resource type policy" {
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "global_value")'
  run_success $OCKAM policy show --resource-type tcp-outlet
  assert_output --partial "tcp-outlet"
  assert_output --partial "(= subject.component \"global_value\")"

  run_success $OCKAM policy show --resource tcp-outlet
  refute_output --partial "tcp-outlet"

  # Will fail if the resource type is not recognized
  run_failure $OCKAM policy show --resource-type not-a-resource-type

  run_success $OCKAM policy delete --resource-type tcp-outlet -y
  run_success $OCKAM policy show --resource-type tcp-outlet
  refute_output --partial "tcp-outlet"
}

@test "policies - create scoped policy" {
  run_success $OCKAM policy create --resource my_policy --expression '(= subject.component "scoped_value")'
  run_success $OCKAM policy show --resource my_policy
  assert_output --partial "my_policy"
  assert_output --partial "(= subject.component \"scoped_value\")"

  run_success $OCKAM policy delete --resource my_policy -y
  run_success $OCKAM policy show --resource my_policy
  refute_output --partial "my_policy"
}
