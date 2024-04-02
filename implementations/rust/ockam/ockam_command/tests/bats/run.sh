#!/bin/bash

rm -rf "$HOME/.bats-tests"

export BATS_TEST_RETRIES=2
export BATS_TEST_TIMEOUT=240

echo "Running local suite..."
bats local --timing -j 8

echo "Running orchestrator suite..."
bats orchestrator --timing

echo "Running serial suite..."
bats serial --timing
