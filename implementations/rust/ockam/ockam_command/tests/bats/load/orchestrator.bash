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

function load_orchestrator_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    cp -a $OCKAM_HOME_BASE $OCKAM_HOME
    export PROJECT_JSON_PATH="$OCKAM_HOME/project.json"
    $OCKAM project information --output json >"$PROJECT_JSON_PATH"
    if [ ! -s "${PROJECT_JSON_PATH}" ]; then
      echo "Project json file is empty" >&3
      exit 1
    fi
  fi
}
