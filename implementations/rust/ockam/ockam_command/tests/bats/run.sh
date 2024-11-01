#!/bin/bash
set -e

rm -rf "$HOME/.bats-tests"
mkdir -p "$HOME/.bats-tests"

export BATS_TEST_TIMEOUT=300
export BATS_TEST_RETRIES=2

current_directory=$(dirname "$0")

# The arguments are the names of the suites to run.
# If no arguments are passed, all suites are run.
if [ $# -eq 0 ]; then
  local_suite=true
  local_as_root_suite=true
  orchestrator_enroll_suite=true
  orchestrator_suite=true
  serial_suite=true
  examples_suite=true
  kafka_suite=false
else
  local_suite=false
  local_as_root_suite=false
  orchestrator_enroll_suite=false
  orchestrator_suite=false
  serial_suite=false
  examples_suite=false
  kafka_suite=false
fi

for suite in "$@"; do
  case $suite in
  local) local_suite=true ;;
  local_as_root) local_as_root_suite=true ;;
  orchestrator_enroll) orchestrator_enroll_suite=true ;;
  orchestrator) orchestrator_suite=true ;;
  serial) serial_suite=true ;;
  examples) examples_suite=true ;;
  kafka) kafka_suite=true ;;
  *)
    echo "Unknown suite: $suite"
    exit 1
    ;;
  esac
done

echo "Running with OCKAM_EBPF=$OCKAM_EBPF"
whoami

if [ "$local_suite" = true ]; then
  echo "Running local suite..."
  bats "$current_directory/local" --timing -j 3
fi

if [ "$local_as_root_suite" = true ]; then
  echo "Running local root suite..."
  OCKAM_EBPF=1 bats "$current_directory/local/portals.bats" --timing -j 3
fi

if [ -z "${ORCHESTRATOR_TESTS}" ]; then
  exit 0
fi

if [ "$orchestrator_enroll_suite" = true ]; then
  echo "Running orchestrator_enroll suite..."
  bats "$current_directory/orchestrator_enroll" --timing
fi

if [ "$orchestrator_suite" = true ]; then
  echo "Running orchestrator suite..."
  bats "$current_directory/orchestrator" --timing
fi

if [ "$serial_suite" = true ]; then
  echo "Running serial suite..."
  bats "$current_directory/serial" --timing
fi

if [ "$examples_suite" = true ]; then
  echo "Running examples suite..."
  bats "$current_directory/examples" --timing
fi

if [ "$kafka_suite" = true ]; then
  echo "Running kafka suite..."
  bats "$current_directory/kafka" --timing --jobs 1
fi
