#!/bin/bash

function skip_if_influxdb_test_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${INFLUXDB_TESTS}" ]; then
    skip "INFLUXDB_TESTS are not enabled"
  fi
}

function skip_if_orchestrator_tests_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi
}

function skip_if_long_tests_not_enabled() {
  if [ -z "${LONG_TESTS}" ]; then
    skip "LONG_TESTS are not enabled"
  fi
}

function load_orchestrator_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    cp -a $OCKAM_HOME_BASE $OCKAM_HOME
    export PROJECT_JSON_PATH="$OCKAM_HOME/project.json"
    $OCKAM project information --output json >"$PROJECT_JSON_PATH"
  fi
}
