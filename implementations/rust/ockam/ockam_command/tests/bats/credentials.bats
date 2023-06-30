#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "credential - issue, verify, store, show and list" {
  run "$OCKAM" identity create i1
  assert_success
  idt1=$($OCKAM identity show i1 --full --encoding hex)

  run "$OCKAM" identity create i2
  assert_success
  idt2=$($OCKAM identity show i2 --full --encoding hex)

  # No "run" here since it won't redirect the output to a file if we do so.
  "$OCKAM" credential issue --as i1 --for "$idt2" --attribute application="Smart Factory" --attribute city="New York" --encoding hex >"$OCKAM_HOME/credential"

  run "$OCKAM" credential verify --issuer "$idt1" --credential-path "$OCKAM_HOME/credential"
  assert_success
  assert_output --partial "true"

  run "$OCKAM" credential store smart_nyc_cred --issuer "$idt1" --credential-path "$OCKAM_HOME/credential"
  assert_success

  run "$OCKAM" credential show smart_nyc_cred
  assert_success
  assert_output --partial "Credential: smart_nyc_cred"
  assert_output --partial "Attributes: {\"application\": \"Smart Factory\", \"city\": \"New York\""

  run "$OCKAM" credential list
  assert_success
  assert_output --partial "Credential: smart_nyc_cred"
  assert_output --partial "Attributes: {\"application\": \"Smart Factory\", \"city\": \"New York\""
}

@test "credential - verify and store reject invalid credentials" {
  run "$OCKAM" identity create i1
  assert_success
  idt1=$($OCKAM identity show i1 --full --encoding hex)

  # create an invalid credential
  echo "FOOBAR" > "$OCKAM_HOME/bad_credential"

  run "$OCKAM" credential verify --issuer "$idt1" --credential-path "$OCKAM_HOME/bad_credential"
  assert_success
  assert_output --partial "false"

  run "$OCKAM" credential store smart_la_cred --issuer "$idt1" --credential-path "$OCKAM_HOME/bad_credential"
  assert_failure
  assert_output --partial "Credential is invalid"

  run "$OCKAM" credential show smart_la_cred
  assert_failure
  assert_output --partial "Unable to find credential named smart_la_cred"
}