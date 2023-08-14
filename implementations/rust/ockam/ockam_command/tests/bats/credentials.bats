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
  run_success "$OCKAM" identity create i1
  idt1=$($OCKAM identity show i1 --full --encoding hex)
  idt1_short=$($OCKAM identity show i1)

  run_success "$OCKAM" identity create i2
  idt2=$($OCKAM identity show i2 --full --encoding hex)
  idt2_short=$($OCKAM identity show i2)

  # No "run" here since it won't redirect the output to a file if we do so.
  "$OCKAM" credential issue --as i1 --for "$idt2_short" --attribute application="Smart Factory" --attribute city="New York" --encoding hex >"$OCKAM_HOME/credential"

  run_success "$OCKAM" credential verify --issuer "$idt1_short" --credential-path "$OCKAM_HOME/credential"
  assert_output --partial "true"

  run_success "$OCKAM" credential store smart_nyc_cred --issuer "$idt1" --credential-path "$OCKAM_HOME/credential"

  run_success "$OCKAM" credential show smart_nyc_cred
  assert_output --partial "Credential: smart_nyc_cred"
  assert_output --partial "{\"application\": \"Smart Factory\", \"city\": \"New York\""

  run_success "$OCKAM" credential list
  assert_output --partial "Credential: smart_nyc_cred"
  assert_output --partial "{\"application\": \"Smart Factory\", \"city\": \"New York\""
}

@test "credential - verify and store reject invalid credentials" {
  run_success "$OCKAM" identity create i1
  idt1=$($OCKAM identity show i1 --full --encoding hex)
  idt1_short=$($OCKAM identity show i1)

  # create an invalid credential
  echo "aabbcc" >"$OCKAM_HOME/bad_credential"

  run_success "$OCKAM" credential verify --issuer "$idt1_short" --credential-path "$OCKAM_HOME/bad_credential"
  assert_output --partial "false"

  run_failure "$OCKAM" credential store smart_la_cred --issuer "$idt1" --credential-path "$OCKAM_HOME/bad_credential"
  assert_output --partial "Credential is invalid"

  run_failure "$OCKAM" credential show smart_la_cred
  assert_output --partial "Unable to find credential named smart_la_cred"
}
