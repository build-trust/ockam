#!/bin/bash

setup_suite() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  orchestrator_setup_suite
}

teardown_suite() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  orchestrator_teardown_suite
}
