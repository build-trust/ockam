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

@test "kafka - CRUD kafka outlet" {
  # We currently support having a single kafka outlet per node
  port="$(random_port)"
  run_success $OCKAM kafka-outlet create --bootstrap-server "127.0.0.1:$port" --jq '.bootstrap_server'
  assert_output --partial "127.0.0.1:$port"

  # Show the outlet
  run_success $OCKAM kafka-outlet show --jq '.'
  assert_output --partial "kafka_outlet"
  run_success $OCKAM kafka-outlet show kafka_outlet --jq '.'
  assert_output --partial "kafka_outlet"

  # List the outlet
  run_success $OCKAM kafka-outlet list --jq '. | length'
  assert_output 1
  # Check the address of the outlet
  run_success $OCKAM kafka-outlet list --jq '.[].addr'
  assert_output --partial "kafka_outlet"

  # Delete the outlet
  run_success $OCKAM kafka-outlet delete kafka_outlet --yes

  # Check that there are no outlets
  run_success $OCKAM kafka-outlet list --jq '. | length'
  assert_output 0
}

@test "kafka - CRUD kafka inlet" {
  run_success $OCKAM kafka-inlet create --to /secure/api
  # Fail to create inlets on the same default addresses
  run_failure $OCKAM kafka-inlet create --to /secure/api
  run_failure $OCKAM kafka-inlet create --to /secure/api --from $(random_port)
  # Create a second inlet
  port="$(random_port)"
  run_success $OCKAM kafka-inlet create inlet2 --to /secure/api --from $port --jq '.'
  assert_output --partial "\"from\": \"127.0.0.1:$port\""
  assert_output --partial "\"to\": \"/secure/api\""

  # Show the inlet
  run_success $OCKAM kafka-inlet show --jq '.'
  assert_output --partial "kafka_inlet"
  run_success $OCKAM kafka-inlet show kafka_inlet --jq '.'
  assert_output --partial "kafka_inlet"

  # List the inlet
  run_success $OCKAM kafka-inlet list --jq '. | length'
  assert_output 2
  # Check the address of the inlets
  run_success $OCKAM kafka-inlet list --jq '.[].addr'
  assert_output --partial "kafka_inlet"
  assert_output --partial "inlet2"

  # Delete the first inlet
  run_success $OCKAM kafka-inlet delete kafka_inlet --yes

  # Check that there is only one inlet left
  run_success $OCKAM kafka-inlet list --jq '. | length'
  assert_output 1
  run_success $OCKAM kafka-inlet list --jq '.[].addr'
  assert_output --partial "inlet2"
}

@test "kafka - create with deprecated alias --addr flag" {
  # Ignore `name` if `addr` is also passed
  port="$(random_port)"
  run_success $OCKAM kafka-inlet create kinlet-name --addr kinlet --to /secure/api --from $port --jq '.'
  assert_output --partial "\"from\": \"127.0.0.1:$port\""
  assert_output --partial "\"to\": \"/secure/api\""

  run_success $OCKAM kafka-inlet show kinlet --jq '.'
  assert_output --partial "kinlet"

  # Use `addr`
  port="$(random_port)"
  run_success $OCKAM kafka-inlet create --addr kinlet2 --to /secure/api --from $port --jq '.'
  assert_output --partial "\"from\": \"127.0.0.1:$port\""
  assert_output --partial "\"to\": \"/secure/api\""

  run_success $OCKAM kafka-inlet show kinlet2 --jq '.'
  assert_output --partial "kinlet2"
}
