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
  port="$(random_port)"

  run "$OCKAM" identity create authority
  run "$OCKAM" identity create enroller
  # m1 will be pre-enrolled on authority.  m2 will be added directly, m3 will be added through enrollment token
  run "$OCKAM" identity create m1
  run "$OCKAM" identity create m2
  run "$OCKAM" identity create m3

  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)
  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  # Start the authority node.  We pass a set of pre trusted-identities containing m1' identity identifier
  # For the first test we start the node with no direct authentication service nor token enrollment
  trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\", \"project_id\" : \"1\", \"trust_context_id\" : \"1\"}, \"$enroller_identifier\": {\"project_id\": \"1\", \"trust_context_id\": \"1\", \"ockam-role\": \"enroller\"}}"
  run "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted" --no-direct-authentication --no-token-enrollment
  assert_success
  sleep 1 # wait for authority to start TCP listener

  PROJECT_JSON_PATH="$OCKAM_HOME/project-authority.json"
  PROJECT_NAME="default"
  echo "{\"id\": \"1\",
  \"name\" : \"$PROJECT_NAME\",
  \"identity\" : \"I6c20e814b56579306f55c64e8747e6c1b4a53d9a\",
  \"access_route\" : \"/dnsaddr/127.0.0.1/tcp/4000/service/api\",
  \"authority_access_route\" : \"/dnsaddr/127.0.0.1/tcp/$port/service/api\",
  \"authority_identity\" : \"$authority_identity_full\"}" >"$PROJECT_JSON_PATH"

  # m1 is a member (its on the set of pre-trusted identifiers) so it can get it's own credential
  run "$OCKAM" project enroll --project-path "$PROJECT_JSON_PATH" --identity m1
  assert_success
  assert_output --partial "sample_val"

  echo "$trusted" >"$OCKAM_HOME/trusted-anchors.json"
  # Restart the authority node with a trusted identities file and check that m1 can still enroll
  run "$OCKAM" node delete authority --yes
  run "$OCKAM" authority create --tcp-listener-address=127.0.0.1:$port --project-identifier 1 --reload-from-trusted-identities-file "$OCKAM_HOME/trusted-anchors.json"
  assert_success
  sleep 1 # wait for authority to start TCP listener

  run "$OCKAM" project ticket --identity enroller --project "$PROJECT_NAME" --member $m2_identifier --attribute sample_attr=m2_member
  assert_success

  run "$OCKAM" project enroll --force --project "$PROJECT_NAME" --identity m2
  assert_success
  assert_output --partial "m2_member"

  token=$($OCKAM project ticket --identity enroller --project "$PROJECT_NAME" --attribute sample_attr=m3_member)
  run "$OCKAM" project enroll --force $token --identity m3
  assert_success
  assert_output --partial "m3_member"
}

@test "authority - enrollment ticket ttl" {
  port="$(random_port)"

  run "$OCKAM" identity create authority
  run "$OCKAM" identity create enroller
  #m3 will be added through enrollment token
  run "$OCKAM" identity create m3

  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)

  # Start the authority node.
  trusted="{\"$enroller_identifier\": {\"project_id\": \"1\", \"trust_context_id\": \"1\", \"ockam-role\": \"enroller\"}}"
  run "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted"
  assert_success
  sleep 1 # wait for authority to start TCP listener

  PROJECT_JSON_PATH="$OCKAM_HOME/project-authority.json"
  echo "{\"id\": \"1\",
  \"name\" : \"default\",
  \"identity\" : \"I6c20e814b56579306f55c64e8747e6c1b4a53d9a\",
  \"access_route\" : \"/dnsaddr/127.0.0.1/tcp/4000/service/api\",
  \"authority_access_route\" : \"/dnsaddr/127.0.0.1/tcp/$port/service/api\",
  \"authority_identity\" : \"$authority_identity_full\"}" >"$PROJECT_JSON_PATH"

  # Enrollment ticket expired by the time it's used
  token=$($OCKAM project ticket --identity enroller --project-path "$PROJECT_JSON_PATH" --attribute sample_attr=m3_member --expires-in 1s)
  sleep 2
  run "$OCKAM" project enroll $token --identity m3
  assert_failure

  # Enrollment ticket with enough ttl
  token=$($OCKAM project ticket --identity enroller --project-path "$PROJECT_JSON_PATH" --attribute sample_attr=m3_member --expires-in 30s)
  run "$OCKAM" project enroll $token --identity m3
  assert_success
  assert_output --partial "m3_member"
}
