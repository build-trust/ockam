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
    if [ ! -d "$HOME/.ockam" ]; then
      echo "Ockam data directory not found: $HOME/.ockam"
      exit 1
    elif [ ! -f "$HOME/.ockam/project.json" ]; then
      OCKAM_HOME="$HOME/.ockam" $OCKAM project information --output json > "$HOME/.ockam/project.json"
    fi
    export PROJECT_JSON_PATH="$HOME/.ockam/project.json"
  fi
}

function copy_orchestrator_data() {
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    if [ ! -d "$HOME/.ockam" ]; then
      echo "Ockam data directory not found: $HOME/.ockam"
      exit 1
    fi
    cp -r "$HOME/.ockam" $OCKAM_HOME
  fi
}
