#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "projects - list" {
  run_success "$OCKAM" project list
}

@test "space - list" {
  run_success "$OCKAM" space list
}

@test "elastic encrypted relays" {
  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" node create "$a"
  run_success "$OCKAM" node create "$b"
  run_success "$OCKAM" relay create "$b" --at /project/default --to "/node/$a"

  run_success bash -c "$OCKAM secure-channel create --from $a --to /project/default/service/forward_to_$b/service/api |
    $OCKAM message send hello --from $a --to -/service/uppercase"
  assert_output "HELLO"
}

@test "managed authorities" {
  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" node create "$a"
  run_success "$OCKAM" node create "$b"

  run_success "$OCKAM" relay create "$b" --at /project/default --to "/node/$a/service/forward_to_$b"

  run_success bash -c "$OCKAM secure-channel create --from $a --to /project/default/service/forward_to_$b/service/api |
    $OCKAM message send hello --from $a --to -/service/uppercase"
  assert_output "HELLO"
}
