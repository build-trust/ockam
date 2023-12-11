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

function copy_local_orchestrator_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    export PROJECT_NAME="default"
    export PROJECT_PATH="$OCKAM_HOME_BASE/project.json"

    # export the project data to a file
    OCKAM_HOME=$OCKAM_HOME_BASE "$OCKAM" project show -q --output json >$PROJECT_PATH

    # import it to the current OCKAM_HOME directory
    cp -r $OCKAM_HOME_BASE/. $OCKAM_HOME
  fi
}
