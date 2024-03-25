#!/bin/bash

setup_suite() {
  load load/base.bash
  mkdir -p $OCKAM_HOME_BASE/.tmp
  setup_python_server
  OCKAM_HOME=$OCKAM_HOME_BASE $OCKAM node delete --all --force --yes
  export BATS_TEST_TIMEOUT=300
}

teardown_suite() {
  load load/base.bash
  teardown_python_server
  rm -rf $OCKAM_HOME_BASE/.tmp
}
