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

@test "portals - fail to create two TCP outlets with the same alias" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  o="$(random_str)"
  port="$(random_port)"
  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"
  assert_success

  port="$(random_port)"
  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"
  assert_failure
}

@test "portals - fail to create two TCP outlets at the same address" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  port="$(random_port)"
  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port"
  assert_success

  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port"
  assert_failure
}

@test "portals - fail to create two TCP inlets with the same alias" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  o="$(random_str)"
  port="$(random_port)"
  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"
  assert_success

  i="$(random_str)"
  port="$(random_port)"
  run "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"
  assert_success

  port="$(random_port)"
  run "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"
  assert_failure
}

@test "portals - fail to create two TCP inlets at the same address" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  o="$(random_str)"
  port="$(random_port)"
  run "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"
  assert_success

  port="$(random_port)"
  run "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"
  assert_success

  run "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"
  assert_failure
}
