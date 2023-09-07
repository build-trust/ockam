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
    cp -a $OCKAM_HOME_BASE $OCKAM_HOME
    export PROJECT_JSON_PATH="$OCKAM_HOME/project.json"
    export PROJECT_NAME="default"
    cp $OCKAM_HOME/projects/default.json $PROJECT_JSON_PATH
  fi
}

function fetch_orchestrator_data() {
  copy_local_orchestrator_data
  max_retries=5
  i=0
  while [[ $i -lt $max_retries ]]; do
    run bash -c "$OCKAM project information --output json >$PROJECT_JSON_PATH"
    # if status is not 0, retry
    if [ $status -ne 0 ]; then
      sleep 5
      ((i++))
      continue
    fi
    # if file is empty, exit with error
    if [ ! -s "$PROJECT_JSON_PATH" ]; then
      echo "Project information is empty" >&3
      exit 1
    fi
    break
  done
  if [ $i -eq $max_retries ]; then
    echo "Failed to fetch project information" >&3
    exit 1
  fi
}
