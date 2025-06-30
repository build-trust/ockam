#!/bin/bash
set -e

rm -rf "$HOME/.bats-tests"
mkdir -p "$HOME/.bats-tests"

export BATS_TEST_TIMEOUT=180

export LOCAL_EXAMPLES_DIR="../../examples"
export MAIN_EXAMPLES_DIR="../../../../examples"

current_directory=$(dirname "$0")

# The arguments are the names of the suites to run.
# If no arguments are passed, all suites are run.
if [ $# -eq 0 ]; then
  examples_suite=true
  examples_orchestrator_suite=true
else
  examples_suite=false
  examples_orchestrator_suite=false
fi

for suite in "$@"; do
  case $suite in
  examples) examples_suite=true ;;
  examples_orchestrator) examples_orchestrator_suite=true ;;
  *)
    echo "Unknown suite: '$suite', ignoring..."
    ;;
  esac
done

if [ "$examples_suite" = true ]; then
  echo "Running examples suite..."
  bats "$current_directory/examples" --timing
fi

if [ -z "${ORCHESTRATOR_TESTS}" ]; then
  exit 0
fi

if [ "$examples_suite" = true ]; then
  echo "Running examples orchestrator suite..."
  bats "$current_directory/examples_orchestrator" --timing
fi
