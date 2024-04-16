#!/bin/bash

setup_suite() {
  load ../load/base.bash
  setup_python_server

  # Remove all nodes from the root OCKAM_HOME directory
  OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM node delete --all --force --yes
}

teardown_suite() {
  load ../load/base.bash
  teardown_python_server
  rm -rf $OCKAM_HOME_BASE/.tmp
}
