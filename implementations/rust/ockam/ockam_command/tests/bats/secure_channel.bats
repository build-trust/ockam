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

@test "secure channel - create secure channel and send message through it" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # In two separate commands
  msg=$(random_str)
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api)
  run_success "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to "$output/service/uppercase"
  assert_output "$(to_uppercase "$msg")"

  # Piping the output of the first command into the second
  msg=$(random_str)
  run_success bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
    | $OCKAM message send $msg --from /node/n1 --to -/service/uppercase"
  assert_output "$(to_uppercase "$msg")"

  # Using an explicit secure channel listener
  run_success "$OCKAM" secure-channel-listener create n2scl --at /node/n2
  msg=$(random_str)
  run_success bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/n2scl \
    | $OCKAM message send $msg --from /node/n1 --to -/service/uppercase"
  assert_output "$(to_uppercase "$msg")"
}

@test "secure channel - send message directly using secure multiaddr" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  msg=$(random_str)
  run_success "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to "/node/n2/secure/api/service/uppercase"
  assert_output "$(to_uppercase "$msg")"
}
