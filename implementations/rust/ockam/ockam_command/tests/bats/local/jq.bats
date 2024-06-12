#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "vault - json + jq" {
  run_success "$OCKAM" vault create v1

  run_success "$OCKAM" vault show v1 --output json --jq .
  assert_output --partial "\"name\":\"v1\""
  assert_output --partial "\"use_aws_kms\":\"No\""

  run_success "$OCKAM" vault show v1 --output json --jq .vault.name
  assert_output --partial "v1"
  run_success "$OCKAM" vault show v1 --jq .vault.name
  assert_output --partial "v1"

  run_success "$OCKAM" vault create v2

  run_success "$OCKAM" vault list --output json --jq 'map(.vault.name) | join(" ")'
  assert_output --partial "v1 v2"
}

@test "node - json + jq" {
  run_success "$OCKAM" node create n

  run_success "$OCKAM" node show n --jq .
  assert_output --partial "\"name\":\"n\""
  assert_output --partial "/dnsaddr/localhost/tcp/"
  assert_output --partial "\"addr\":\"uppercase\""

  run_success "$OCKAM" node show n --jq .name
  assert_output --partial "n"
}
