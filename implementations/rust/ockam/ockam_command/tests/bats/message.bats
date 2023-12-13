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

@test "message - send messages between local nodes" {
  # Send from a temporary node to a background node
  run_success "$OCKAM" node create n1
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --timeout 5 --to /node/n1/service/uppercase
  assert_output "$(to_uppercase "$msg")"

  # Send between two background nodes
  run_success "$OCKAM" node create n2
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --timeout 5 --from n1 --to /node/n2/service/uppercase
  assert_output "$(to_uppercase "$msg")"

  # Same, but using the `/node/` prefix in the `--from` argument
  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to /node/n2/service/uppercase
  assert_output "$(to_uppercase "$msg")"
}

@test "message - secure-channels with authorized identifiers" {
  run_success "$OCKAM" vault create v1
  run_success "$OCKAM" identity create i1 --vault v1
  idt1=$($OCKAM identity show i1)

  run_success "$OCKAM" vault create v2
  run_success "$OCKAM" identity create i2 --vault v2
  idt2=$($OCKAM identity show i2)

  run_success "$OCKAM" node create n1 --identity i1
  run_success "$OCKAM" node create n2 --identity i1

  msg=$(random_str)
  run_success "$OCKAM" secure-channel-listener create l --at n2 --identity i2 --authorized "$idt1"
  run_success bash -c "$OCKAM secure-channel create --from n1 --to /node/n2/service/l --authorized $idt2 \
              | $OCKAM message send $msg --from /node/n1 --to -/service/echo"
  assert_output "$msg"
}
