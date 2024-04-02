#!/bin/bash

# https://docs.ockam.io/use-cases

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
  load ../load/orchestrator.bash
  skip_if_orchestrator_tests_not_enabled
  load ../load/docs.bash
  skip_if_docs_tests_not_enabled
  get_project_data
  copy_enrolled_home_dir
}

teardown() {
  kill_kafka_contents || true
  kill_flask_server || true
  kill_telegraf_instance || true
  teardown_home_dir
}

# ===== TESTS

# https://docs.ockam.io/
# Please update the docs repository if this bats test is updated
@test "use-case - end-to-end encryption, orchestrator" {
  inlet_port="$(random_port)"
  relay_name="$(random_str)"

  # Service
  run_success "$OCKAM" tcp-outlet create --to $PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create $relay_name

  # Client
  run_success $OCKAM tcp-inlet create --from "$inlet_port" --via "$relay_name"
  run_success curl --fail --head --retry-connrefused --retry-delay 5 --retry 10 --max-time 5 "127.0.0.1:$inlet_port"
}

# https://docs.ockam.io/guides/examples/create-secure-communication-with-a-private-database-from-anywhere
# Please update the docs repository if this bats test is updated
@test "use-case - create-secure-communication-with-a-private-database-from-anywhere" {
  skip "createdb function does not exist"
  export PGHOST="$PG_HOST"
  export PGPASSWORD="password"
  run_success createdb -U postgres app_db

  run_success "$OCKAM" tcp-outlet create --to "$PG_HOST:$PG_PORT"
  run_success "$OCKAM" relay create

  run_success $OCKAM tcp-inlet create --from 7777
  # Call the list database -l
  run_success psql --host="127.0.0.1" --port=7777 -U postgres app_db -l
}
