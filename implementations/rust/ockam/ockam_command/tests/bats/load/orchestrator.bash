#!/bin/bash

function exit_if_orchestrator_tests_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    exit 0
  fi
}

function skip_if_orchestrator_tests_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi
}

function skip_if_influxdb_test_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${INFLUXDB_TESTS}" ]; then
    skip "INFLUXDB_TESTS are not enabled"
  fi
}

function get_project_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    export PROJECT_NAME="default"
    export PROJECT_PATH="$BATS_SUITE_TMPDIR/project.json"
    OCKAM_HOME=$OCKAM_HOME_BASE "$OCKAM" project show -q --output json >$PROJECT_PATH
  fi
}

function copy_enrolled_home_dir() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    cp -r $OCKAM_HOME_BASE/application_database.sqlite3 $OCKAM_HOME/
    cp -r $OCKAM_HOME_BASE/database.sqlite3 $OCKAM_HOME/
  fi
}
