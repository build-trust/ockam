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
  assert_success DEFAULT_PORT_HTTP="$(random_port)" \
    OCKAM_SQLITE_IN_MEMORY=1 \
    uv run "$EXAMPLES_DIR"/01.py
}
