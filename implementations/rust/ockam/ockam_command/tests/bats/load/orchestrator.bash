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
    if [ ! -f "$OCKAM_HOME_BASE/project.json" ]; then
      OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM project information --output json >"$OCKAM_HOME_BASE/project.json"
    fi
    export PROJECT_JSON_PATH="$OCKAM_HOME_BASE/project.json"
  fi
}

function copy_orchestrator_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    cp -a $OCKAM_HOME_BASE $OCKAM_HOME
  fi
}
