#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/base_python.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "example 01" {
  skip
  check_file_exists "$LOCAL_EXAMPLES_DIR"/01.py
  run_success bash -c "DEFAULT_PORT_HTTP=$(random_port) \
    OCKAM_SQLITE_IN_MEMORY=1 \
    OCKAM_LOGGING=0 \
    uv run $LOCAL_EXAMPLES_DIR/01.py"
  assert_output "test"
}

@test "example 02" {
  skip
  check_file_exists "$LOCAL_EXAMPLES_DIR"/02.py
  run_success bash -c "DEFAULT_PORT_HTTP=$(random_port) \
    OCKAM_SQLITE_IN_MEMORY=1 \
    OCKAM_LOGGING=0 \
    uv run $LOCAL_EXAMPLES_DIR/02.py"
  assert_output "hello"
}

@test "example 03" {
  skip
  check_file_exists "$LOCAL_EXAMPLES_DIR"/03.py
  run_success bash -c "DEFAULT_PORT_HTTP=$(random_port) \
    OCKAM_SQLITE_IN_MEMORY=1 \
    OCKAM_LOGGING=0 \
    uv run $LOCAL_EXAMPLES_DIR/03.py"
  assert_output "hello"
}
