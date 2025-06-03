#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/base_python.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "example 04 - agent to agent communication over a secure channel" {
  skip
  export ECHO_NODE="node-$(random_hex_str)"
  export CLIENT_NODE="node-$(random_hex_str)"

  ENROLLMENT_TICKET="$($OCKAM cluster ticket --relay $ECHO_NODE --zone $ZONE_NAME)" \
  DEFAULT_PORT_HTTP="$(random_port)" \
  CLUSTER=$CLUSTER \
  NODE=$ECHO_NODE \
  OCKAM_SQLITE_IN_MEMORY=1 \
    uv run "$EXAMPLES_DIR"/04-echo.py &
  sleep 1

  run_success ENROLLMENT_TICKET="$($OCKAM cluster ticket --relay $ECHO_NODE --zone $ZONE_NAME)" \
    DEFAULT_PORT_HTTP="$(random_port)" \
    CLUSTER=$CLUSTER \
    NODE=$CLIENT_NODE \
    OCKAM_SQLITE_IN_MEMORY=1 \
    uv run "$EXAMPLES_DIR"/04-client.py $ECHO_NODE

  # the output must contain the string "hello" returned from the echo node.
  assert_output --partial "hello"
}
