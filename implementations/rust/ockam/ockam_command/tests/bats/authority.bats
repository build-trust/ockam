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

@test "authority - standalone authority, enrollers, members" {
  run "$OCKAM" identity create authority
  run "$OCKAM" identity create enroller

  # m1 will be pre-authenticated on authority; m2 will be added directly; m3 will be added through enrollment token
  run "$OCKAM" identity create m1
  run "$OCKAM" identity create m2
  run "$OCKAM" identity create m3
  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show authority --full --encoding hex)
  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  # To startup an authority we need some boilerplate setup in place.
  # Create an enrollers file with the identity of the enroller we want.
  echo "{\"$enroller_identifier\": {}}" > "$OCKAM_HOME/enrollers.json"
  # Create a launch configuration json file, to be used to start the authority node
  echo "{\"startup_services\": {
    \"authenticator\": {\"enrollers\" : \"$OCKAM_HOME/enrollers.json\", \"project\" : \"1\"},
    \"secure_channel_listener\": {}
  }}" > "$OCKAM_HOME/auth_launch_config.json"

  # Start the authority node. We pass a set of pre trusted-identities containing m1's identity identifier
  run "$OCKAM" node create --tcp-listener-address=0.0.0.0:4200 --identity authority --launch-config "$OCKAM_HOME/auth_launch_config.json" --trusted-identities "{\"$m1_identifier\": {\"sample_attr\" : \"sample_val\"}}"  authority
  assert_success

  echo "{\"id\": \"1\",
    \"name\" : \"default\",
    \"identity\" : \"P6c20e814b56579306f55c64e8747e6c1b4a53d9a3f4ca83c252cc2fbfc72fa94\",
    \"access_route\" : \"/dnsaddr/127.0.0.1/tcp/4000/service/api\",
    \"authority_access_route\" : \"/dnsaddr/127.0.0.1/tcp/4200/service/api\",
    \"authority_identity\" : \"$authority_identity_full\"}" > "$OCKAM_HOME/project.json"

  # m1 is a member (its on the set of pre-trusted identifiers) so it can get it's own credential
  run "$OCKAM" project authenticate --project-path "$OCKAM_HOME/project.json" --identity m1
  assert_success
  assert_output --partial "sample_val"

  run "$OCKAM" project enroll --identity enroller --project-path "$OCKAM_HOME/project.json" --member "$m2_identifier" --attribute sample_attr=m2_member
  assert_success

  run "$OCKAM" project authenticate --project-path "$OCKAM_HOME/project.json" --identity m2
  assert_success
  assert_output --partial "m2_member"

  token=$($OCKAM project enroll --identity enroller --project-path "$OCKAM_HOME/project.json"  --attribute sample_attr=m3_member)
  run "$OCKAM" project authenticate --project-path "$OCKAM_HOME/project.json" --identity m3  --token "$token"
  assert_success
  assert_output --partial "m3_member"
}
