#!/bin/bash

setup_suite() {
  load ../load/base.bash
  load ../load/base_python.bash
  load ../load/orchestrator.bash
  load ../load/orchestrator_python.bash
  orchestrator_python_setup_suite
}

teardown_suite() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load ../load/orchestrator_python.bash
  orchestrator_python_teardown_suite
}
