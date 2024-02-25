#!/bin/bash

# ===== SETUP

setup_file() {
  load load/base.bash
}

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "policies - inlet/outlet with resource type policies" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --relay $relay_name)
  web_ticket=$($OCKAM project ticket --attribute component=web)
  dashboard_ticket=$($OCKAM project ticket --attribute component=dashboard)

  # DB
  setup_home_dir
  DB_OCKAM_HOME=$OCKAM_HOME
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "web")'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp - Has the right attribute, so it should be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name
  run_success curl --head --retry-connrefused --retry 2 --max-time 5 "127.0.0.1:$inlet_port"

  # Dashboard - Doesn't have the right attribute, so it should not be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $dashboard_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name
  run_failure curl --head --retry-connrefused --max-time 5 "127.0.0.1:$inlet_port"
}

@test "policies - inlet/outlet with resource type policies override" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --relay $relay_name)
  web_ticket=$($OCKAM project ticket --attribute component=web)

  # DB
  setup_home_dir
  DB_OCKAM_HOME=$OCKAM_HOME
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  ### Set wrong resource type policy
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "NOT_web")'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name

  # This will fail because the resource type policy is not satisfied
  run_failure curl --head --retry-connrefused --max-time 3 "127.0.0.1:$inlet_port"

  # Update resource type policy and try again. Now the policy is satisfied
  export OCKAM_HOME=$DB_OCKAM_HOME
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "web")'
  run_success curl --head --retry-connrefused --retry 2 --max-time 5 "127.0.0.1:$inlet_port"

  # Update the policy for the outlet and try again. It will fail because the local policy is not satisfied
  run_success $OCKAM policy create --resource outlet --expression '(= subject.component "NOT_web")'
  run_failure curl --head --retry-connrefused --max-time 3 "127.0.0.1:$inlet_port"
}
