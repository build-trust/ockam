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

@test "policies - create resource type policy, backwards compatibility" {
  run_success $OCKAM policy create --resource tcp-outlet --expression '(= subject.component "global_value")'
  run_success $OCKAM policy show tcp-outlet
  assert_output --partial "tcp-outlet"
  assert_output --partial '(= subject.component \"global_value\")'

  run_success $OCKAM policy delete tcp-outlet -y
  run_success $OCKAM policy show tcp-outlet
  refute_output --partial "tcp-outlet"
}

@test "policies - create resource type policy" {
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "global_value")'
  run_success $OCKAM policy show tcp-outlet
  assert_output --partial "tcp-outlet"
  assert_output --partial '(= subject.component \"global_value\")'

  # Will fail if the resource type is not recognized
  run_failure $OCKAM policy showype not-a-resource-type

  run_success $OCKAM policy delete tcp-outlet -y
  run_success $OCKAM policy show tcp-outlet
  refute_output --partial "tcp-outlet"
}

@test "policies - create scoped policy" {
  run_success $OCKAM policy create --resource my_policy --expression '(= subject.component "scoped_value")'
  run_success $OCKAM policy show my_policy
  assert_output --partial "my_policy"
  assert_output --partial '(= subject.component \"scoped_value\")'

  run_success $OCKAM policy delete my_policy -y
  run_success $OCKAM policy show my_policy
  refute_output --partial "my_policy"
}

@test "policies - create policy with a boolean expression" {
  run_success $OCKAM policy create --resource my_policy --allow 'component.db or component.web'
  run_success $OCKAM policy show my_policy
  assert_output --partial "my_policy"
  assert_output --partial '"expression": "(or (= subject.component.db \"true\") (= subject.component.web \"true\"))"'
}

@test "policies - create policy with an incorrect policy expression" {
  run_failure $OCKAM policy create --resource my_policy --allow 'component.db or'
  assert_output --partial "invalid value 'component.db or'"
  assert_output --partial 'successfully parsed: `component.db`, but ` or` cannot be parsed'
}
