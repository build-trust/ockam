#!/bin/bash

# ===== SETUP

setup_file() {
  load load/base.bash
}

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "no trust context: all authorized" {
  port=8001
  $OCKAM identity create alice
  $OCKAM identity create bob
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port
  msg=$(random_str)
  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo $msg
  assert_success
  assert_output "$msg"
}

@test "trust context: pre-trusted config, authorized" { 
  port=8002
  $OCKAM identity create alice
  $OCKAM identity create bob
  bob_id=$($OCKAM identity show bob)
  trust_context="{\"id\": \"test-context\"}"
  alice_trust_anchors="{\"$bob_id\": {\"project_id\" : \"test-context\"}}"
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "$trust_context" --trusted-identities "$alice_trust_anchors"
  msg=$(random_str)
  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  $msg
  assert_success
  assert_output "$msg"
}

@test "trust context: pre-trusted config, rejected" { 
  port=8003
  $OCKAM identity create alice
  $OCKAM identity create bob
  $OCKAM identity create attacker
  bob_id=$($OCKAM identity show bob)
  trust_context="{\"id\": \"test-context\"}"
  alice_trust_anchors="{\"$bob_id\": {\"project_id\" : \"test-context\"}}"
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "$trust_context" --trusted-identities "$alice_trust_anchors"
  msg=$(random_str)
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  $msg
  assert_failure
}

@test "trust context: offline authority, authorized" {
  port=8004
  $OCKAM identity create alice
  $OCKAM identity create bob
  $OCKAM identity create authority
  bob_id=$($OCKAM identity show bob)
  alice_id=$($OCKAM identity show alice)
  authority_identity=$($OCKAM identity show --full --encoding hex  authority)

  $OCKAM credential issue --as authority --for $bob_id --attribute project_id=test-context --encoding hex > "$OCKAM_HOME/bob.cred"
  $OCKAM credential issue --as authority --for $alice_id --attribute project_id=test-context --encoding hex > "$OCKAM_HOME/alice.cred"
  alice_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/alice.cred\" }}}"
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "$alice_trust_context"


  bob_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/bob.cred\" }}}"
  msg=$(random_str)
  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  --trust-context "$bob_trust_context" $msg
  assert_success
  assert_output "$msg"
}

@test "trust context: offline authority, rejected" {
  port=8005
  $OCKAM identity create alice
  $OCKAM identity create bob
  $OCKAM identity create attacker
  $OCKAM identity create authority
  bob_id=$($OCKAM identity show bob)
  alice_id=$($OCKAM identity show alice)
  attacker_id=$($OCKAM identity show alice)
  authority_identity=$($OCKAM identity show --full --encoding hex  authority)

  $OCKAM credential issue --as authority --for $bob_id --attribute project_id=test-context --encoding hex > "/$OCKAM_HOME/bob.cred"
  $OCKAM credential issue --as authority --for $alice_id --attribute project_id=test-context --encoding hex > "/$OCKAM_HOME/alice.cred"
  alice_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/alice.cred\" }}}"
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "$alice_trust_context"


  msg=$(random_str)

  # Fail, attacker won't present any credential
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  $msg
  assert_failure

  # Fail, attacker will present an invalid credential (self signed rather than signed by authority) 
  $OCKAM credential issue --as attacker --for $attacker_id --attribute project_id=test-context --encoding hex > "$OCKAM_HOME/attacker.cred"
  attacker_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/attacker.cred\" }}}"
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  --trust-context "$attacker_trust_context" $msg
  assert_failure


  # Fail, attacker will present an invalid credential (bob' credential, not own)
  attacker_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/bob.cred\" }}}"
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  --trust-context "$attacker_trust_context" $msg
  assert_failure
 

  # Fail,  *alice* is not presenting credential to bob, required by bob
  port=8006
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "{\"id\" : \"test-context\"}"
  bob_trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"File\" : \"$OCKAM_HOME/bob.cred\" }}}"
  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  --trust-context "$bob_trust_context" $msg
  assert_failure
}

@test "trust context: online authority, authorized" {
  port=8007
  $OCKAM identity create alice
  $OCKAM identity create bob
  $OCKAM identity create authority
  bob_id=$($OCKAM identity show bob)
  alice_id=$($OCKAM identity show alice)
  authority_identity=$($OCKAM identity show --full --encoding hex  authority)

  trusted="{\"$bob_id\": {}, \"$alice_id\": {}}"
  $OCKAM authority create --identity authority --tcp-listener-address=127.0.0.1:4200 --project-identifier "test-context" --trusted-identities "$trusted" 

  trust_context="{\"id\": \"test-context\", \"authority\" : {\"identity\" : \"$authority_identity\", \"credential_retriever\" :{\"Online\" : \"/dnsaddr/127.0.0.1/tcp/4200/service/api\" }}}"
  "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$port --trust-context "$trust_context"


  msg=$(random_str)
  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo  --trust-context "$trust_context" $msg
  assert_success
  assert_output "$msg"
}
