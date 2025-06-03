#!/bin/bash

function orchestrator_python_setup_suite() {
  export OCKAM_COMMAND_RETRY_COUNT=3
  export OCKAM_COMMAND_RETRY_DELAY=1s
  $OCKAM zone create --zone bats-zone || true
  export CLUSTER=$($OCKAM cluster show --jq '.cluster')
  export ZONE_NAME=bats-zone
}

function orchestrator_python_teardown_suite() {
  $OCKAM zone delete --all --yes || true
  rm -rf $OCKAM_HOME_BASE/.tmp
}
