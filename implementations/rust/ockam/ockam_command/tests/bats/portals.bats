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

@test "portals - tcp inlet CRUD" {
  outlet_port="$(random_port)"
  inlet_port="$(random_port)"

  # Create nodes for inlet/outlet pair
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # Create inlet/outlet pair
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$outlet_port" --alias "test-outlet"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$inlet_port --to /node/n1/service/outlet --alias "test-inlet"
  run_success $OCKAM tcp-inlet create --at /node/n2 --from 6102 --to /node/n1/service/outlet

  sleep 1

  # Check that inlet is available for deletion and delete it
  run_success $OCKAM tcp-inlet show test-inlet --at /node/n2 --output json
  assert_output --partial "\"alias\":\"test-inlet\""
  assert_output --partial "\"bind_addr\":\"127.0.0.1:$inlet_port\""

  run_success $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes

  # Test deletion of a previously deleted TCP inlet
  run_failure $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes
  assert_output --partial "not found"
}

@test "portals - tcp outlet CRUD" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  only_port="$(random_port)"
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet create --at /node/n2 --to $only_port

  run_success $OCKAM tcp-outlet show test-outlet --at /node/n1
  assert_output --partial "\"alias\":\"test-outlet\""
  assert_output --partial "\"addr\":\"/service/outlet\""
  assert_output --partial "\"socket_addr\":\"127.0.0.1:$port\""

  run_success $OCKAM tcp-outlet delete "test-outlet" --yes

  # Test deletion of a previously deleted TCP outlet
  run_failure $OCKAM tcp-outlet delete "test-outlet" --yes
  assert_output --partial "not found"
}

@test "portals - list inlets on a node" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias tcp-inlet-2
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list outlets on a node" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet list --at /node/n1
  assert_output --partial "test-outlet"
  assert_output --partial "/service/outlet"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - show a tcp inlet" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias "test-inlet"
  sleep 1

  run_success $OCKAM tcp-inlet show "test-inlet" --at /node/n2

  # Test if non-existing TCP inlet returns NotFound
  run_failure $OCKAM tcp-inlet show "non-existing-inlet"
  assert_output --partial "not found"
}

@test "portals - show a tcp outlet" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet show "test-outlet"

  # Test if non-existing TCP outlet returns NotFound
  run_failure $OCKAM tcp-outlet show "non-existing-outlet"
  assert_output --partial "not found"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to 127.0.0.1:5000
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "127.0.0.1:$port" --to /node/n1/service/outlet

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - create an inlet/outlet pair with relay through a relay and move tcp traffic through it" {
  port="$(random_port)"
  run_success "$OCKAM" node create relay
  run_success "$OCKAM" node create blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  run_success "$OCKAM" relay create blue --at /node/relay --to /node/blue

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"

  run_success "$OCKAM" secure-channel list --at green
  assert_output --partial "/service"
}

@test "portals - fail to create two TCP outlets with the same alias" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  o="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"

  port="$(random_port)"
  run_failure "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"
}

@test "portals - fail to create two TCP outlets at the same address" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port"

  run_failure "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port"
}

@test "portals - fail to create two TCP inlets with the same alias" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  o="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"

  i="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"

  port="$(random_port)"
  run_failure "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"
}

@test "portals - fail to create two TCP inlets at the same address" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  o="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --from /service/outlet --to "127.0.0.1:$port" --alias "$o"

  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"

  run_failure "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"
}
