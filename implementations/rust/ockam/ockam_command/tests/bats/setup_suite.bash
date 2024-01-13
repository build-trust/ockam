#!/bin/bash

setup_suite() {
  load load/base.bash
  setup_python_server

  export BATS_TEST_TIMEOUT=300

  # If we're running orchestrator tests, export the project data into `BATS_SUITE_TMPDIR`
  if [ ! -z "${ORCHESTRATOR_TESTS}" ]; then
    export PROJECT_NAME="default"
    export PROJECT_PATH="$BATS_SUITE_TMPDIR/project.json"
    OCKAM_HOME=$OCKAM_HOME_BASE "$OCKAM" project show -q --output json >$PROJECT_PATH
  fi

  # Remove all nodes from the root OCKAM_HOME directory
  OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM node delete --all --force --yes
}

teardown_suite() {
  load load/base.bash
  teardown_python_server
}
