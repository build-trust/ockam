#!/bin/bash

setup_suite() {
  export OCKAM_COMMAND_RETRY_COUNT=3
  export OCKAM_COMMAND_RETRY_DELAY=5s

  load ../load/base.bash
  load ../load/orchestrator.bash
  exit_if_orchestrator_tests_not_enabled

  mkdir -p $OCKAM_HOME_BASE/.tmp
  setup_python_server
  get_project_data

  # Remove all project members except for the enrolled identity
  OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM project-member delete --all

  # Remove all nodes from the root OCKAM_HOME directory
  OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM node delete --all --force --yes
}

teardown_suite() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  exit_if_orchestrator_tests_not_enabled

  teardown_python_server
  rm -rf $OCKAM_HOME_BASE/.tmp
}
