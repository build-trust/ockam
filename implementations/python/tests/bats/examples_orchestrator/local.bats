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
  check_file_exists "$LOCAL_EXAMPLES_DIR"/04-echo.py
  check_file_exists "$LOCAL_EXAMPLES_DIR"/04-client.py

  export ECHO_NODE="node-$(random_hex_str)"
  export CLIENT_NODE="node-$(random_hex_str)"

  bash -c "ENROLLMENT_TICKET=$($OCKAM zone ticket --relay $ECHO_NODE --zone $ZONE_NAME) \
    DEFAULT_PORT_HTTP=8002 \
    CLUSTER=$CLUSTER \
    NODE=$ECHO_NODE \
    OCKAM_SQLITE_IN_MEMORY=1 \
    OCKAM_LOGGING=0 \
    uv run ./examples/04-echo.py &"
  sleep 1

  run_success bash -c "ENROLLMENT_TICKET=$($OCKAM zone ticket --relay $ECHO_NODE --zone $ZONE_NAME) \
    DEFAULT_PORT_HTTP=8001 \
    CLUSTER=$CLUSTER \
    NODE=$CLIENT_NODE \
    OCKAM_SQLITE_IN_MEMORY=1 \
    OCKAM_LOGGING=0 \
    uv run $LOCAL_EXAMPLES_DIR/04-client.py $ECHO_NODE"
  assert_output --partial "hello"
}
