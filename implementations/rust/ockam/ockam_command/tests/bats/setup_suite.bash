#!/bin/bash

setup_suite() {
  load load/base.bash
  setup_python_server
}

teardown_suite() {
  load load/base.bash
  teardown_python_server
}
