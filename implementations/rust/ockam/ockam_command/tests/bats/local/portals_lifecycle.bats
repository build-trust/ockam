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

@test "portals - create tcp outlet on implicit default node" {
  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"
}

@test "portals - create tcp outlet" {
  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port" --from "test-outlet"
  assert_output --partial "/service/test-outlet"

  # The first outlet that is created without `--from` flag should be named `outlet`
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"

  # After that, the next outlet should be randomly named
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  refute_output --partial "/service/outlet"
}

@test "portals - tcp inlet CRUD" {
  # Create nodes for inlet/outlet pair
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # Create inlet/outlet pair
  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"

  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create "test-inlet" --at /node/n2 --from 127.0.0.1:$inlet_port --to /node/n1/service/outlet
  run_success $OCKAM tcp-inlet create --at /node/n2 --from 6102 --to /node/n1/service/outlet

  sleep 1

  # Check that inlet is available for deletion and delete it
  run_success $OCKAM tcp-inlet show test-inlet --at /node/n2 --output json
  assert_output --partial "\"alias\": \"test-inlet\""
  assert_output --partial "\"bind_addr\": \"127.0.0.1:$inlet_port\""

  run_success $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes

  # Test deletion of a previously deleted TCP inlet
  run_failure $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes
  assert_output --partial "not found"
}

@test "portals - tcp outlet CRUD" {
  run_success "$OCKAM" node create n1

  run_success "$OCKAM" node create n2

  port_1="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port_1"
  assert_output --partial "/service/outlet"

  port_2="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n2 --to $port_2

  run_success $OCKAM tcp-outlet show outlet --at /node/n1
  assert_output --partial "\"worker_address\": \"/service/outlet\""
  assert_output --partial "\"to\": \"127.0.0.1:$port_1\""

  run_success $OCKAM tcp-outlet delete "outlet" --yes

  # Test deletion of a previously deleted TCP outlet
  run_success $OCKAM tcp-outlet delete "outlet" --yes
  assert_output --partial "[]"
}

@test "portals - list inlets on a node" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create tcp-inlet-2 --at /node/n2 --from $port --to /node/n1/service/outlet
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list inlets on a node, using deprecated --alias flag" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create --at /node/n2 --from $port --to /node/n1/service/outlet --alias tcp-inlet-2
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list inlets on a node, using deprecated --alias flag overriding name" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create my-inlet --at /node/n2 --from $port --to /node/n1/service/outlet --alias tcp-inlet-2
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list outlets on a node" {
  run_success "$OCKAM" node create n1

  port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet list --at /node/n1
  assert_output --partial "/service/outlet"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - show a tcp inlet" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create "test-inlet" --at /node/n2 --from $port --to /node/n1/service/outlet
  sleep 1

  run_success $OCKAM tcp-inlet show "test-inlet" --at /node/n2

  # Test if non-existing TCP inlet returns NotFound
  run_failure $OCKAM tcp-inlet show "non-existing-inlet" --at /node/n2
  assert_output --partial "not found"
}

@test "portals - show a tcp outlet" {
  run_success "$OCKAM" node create n1

  port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet show "outlet"

  # Test if non-existing TCP outlet returns NotFound
  run_failure $OCKAM tcp-outlet show "non-existing-outlet"
  assert_output --partial "not found"
}
