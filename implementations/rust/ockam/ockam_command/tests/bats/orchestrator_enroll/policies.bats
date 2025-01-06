#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "policies - inlet/outlet with resource type policies" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --usage-count 10 --relay $relay_name)
  web_ticket=$($OCKAM project ticket --usage-count 10 --attribute component.web)
  dashboard_ticket=$($OCKAM project ticket --usage-count 10 --attribute component.dashboard)

  # DB
  setup_home_dir
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  run_success $OCKAM policy create --resource-type tcp-outlet --allow 'component.web'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp - Has the right attribute, so it should be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --via $relay_name
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  # Dashboard - Doesn't have the right attribute, so it should not be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $dashboard_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --via $relay_name
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"
}

@test "policies - inlet/outlet with resource type policies override" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --usage-count 10 --relay $relay_name)
  web_ticket=$($OCKAM project ticket --usage-count 10 --attribute component.web)

  # DB
  setup_home_dir
  DB_OCKAM_HOME=$OCKAM_HOME
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  ### Set wrong resource type policy
  run_success $OCKAM policy create --resource-type tcp-outlet --allow 'component.NOT_web'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --via $relay_name

  # This will fail because the resource type policy is not satisfied
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"

  # Update resource type policy and try again. Now the policy is satisfied
  export OCKAM_HOME=$DB_OCKAM_HOME
  run_success $OCKAM policy create --resource-type tcp-outlet --allow 'component.web'
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  # Update the policy for the outlet and try again. It will fail because the local policy is not satisfied
  run_success $OCKAM policy create --resource-type tcp-outlet --allow 'component.NOT_web'
  sleep 2
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"
}
