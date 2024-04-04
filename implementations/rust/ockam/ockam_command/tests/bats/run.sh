#!/bin/bash

rm -rf "$HOME/.bats-tests"
mkdir -p "$HOME/.bats-tests"

export BATS_TEST_RETRIES=2
export BATS_TEST_TIMEOUT=240
current_directory=$(dirname "$0")

echo "Running local suite..."
bats "$current_directory/local" --timing -j 8

echo "Running orchestrator suite..."
bats "$current_directory/orchestrator" --timing

echo "Running serial suite..."
bats "$current_directory/serial" --timing
