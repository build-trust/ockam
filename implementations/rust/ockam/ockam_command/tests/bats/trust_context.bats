#!/bin/bash

# ===== SETUP

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

@test "trust_context - CRUD" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  # Create with random name
  run "$OCKAM" trust-context create
  assert_success

  # Create with specific name
  t=$(random_str)
  run "$OCKAM" trust-context create "${t}"
  assert_success

  # List
  run "$OCKAM" trust-context list
  assert_success
  assert_output --partial "${t}"

  # Change the default
  run "$OCKAM" trust-context default "${t}"
  assert_success
  run "$OCKAM" trust-context show
  assert_success
  assert_output --partial "${t}"

  # Delete and verify
  run "$OCKAM" trust-context delete "${t}"
  assert_success
  run "$OCKAM" trust-context show "${t}"
  assert_failure
}

@test "trust context - no trust context; everything is accepted" {
  run "$OCKAM" identity create m1
  run "$OCKAM" node create n1 --identity m1

  run "$OCKAM" identity create m2
  run "$OCKAM" node create n2 --identity m2

  run bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
  assert_success
}

@test "trust context - trust context with an id only; ABAC rules are applied" {
  run "$OCKAM" identity create m1

  echo "{
        \"id\": \"1\"
    }" >"$OCKAM_HOME/trust_context.json"

  m1_identifier=$(run "$OCKAM" identity show m1)
  trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\", \"project_id\" : \"1\", \"trust_context_id\" : \"1\"}}"

  run "$OCKAM" node create n1 --identity m1

  run "$OCKAM" node create n2 --trust-context "$OCKAM_HOME/trust_context.json" --trusted-identities "$trusted"

  run bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
  assert_success

  run "$OCKAM" message send hello --timeout 2 --from /node/n1 --to /node/n2/service/echo
  assert_failure
}

@test "trust context - trust context with an offline authority; Credential Exchange is performed" {
  port="$(random_port)"
  # Create two identities
  run "$OCKAM" identity create alice
  alice_identity=$($OCKAM identity show alice --full --encoding hex)

  run "$OCKAM" identity create bob
  bob_identity=$($OCKAM identity show bob --full --encoding hex)

  $OCKAM identity create attacker

  # Create an identity that both alice and bob will trust
  run "$OCKAM" identity create authority
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # issue and store credentials for alice
  $OCKAM credential issue --as authority --for $alice_identity --attribute city="New York" --encoding hex >"$OCKAM_HOME/alice.cred"
  run "$OCKAM" credential store alice-cred --issuer $authority_identity --credential-path "$OCKAM_HOME/alice.cred"
  $OCKAM trust-context create alice-trust-context --credential alice-cred

  # issue and store credential for bob
  $OCKAM credential issue --as authority --for $bob_identity --attribute city="New York" --encoding hex >"$OCKAM_HOME/bob.cred"
  run "$OCKAM" credential store bob-cred --issuer $authority_identity --credential-path "$OCKAM_HOME/bob.cred"
  $OCKAM trust-context create bob-trust-context --credential bob-cred

  # Create a node for alice that trust authority as a credential authority
  run "$OCKAM" node create alice --tcp-listener-address "127.0.0.1:$port" --identity alice --trust-context alice-trust-context

  msg=$(random_str)

  # Fail, attacker won't present any credential
  run $OCKAM message send --timeout 2 --identity attacker --to "/dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo" $msg
  assert_failure

  # Fail, attacker will present an invalid credential (self signed rather than signed by authority)
  $OCKAM credential issue --as attacker --for $($OCKAM identity show attacker --full --encoding hex) --encoding hex >"$OCKAM_HOME/attacker.cred"
  $OCKAM credential store att-cred --issuer $authority_identity --credential-path $OCKAM_HOME/attacker.cred
  $OCKAM trust-context create att-trust-context --credential att-cred

  run $OCKAM message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context att-trust-context $msg
  assert_failure

  # Fail, attacker will present an invalid credential (bob' credential, not own)
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg
  assert_failure

  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg
  assert_success
  assert_output $msg

  $OCKAM node delete alice
  echo "{\"id\": \"$authority_id\"}" >"$OCKAM_HOME/alice-trust-context.json"
  $OCKAM node create alice --tcp-listener-address 127.0.0.1:$port --identity alice --trust-context "$OCKAM_HOME/alice-trust-context.json"

  run "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg
  assert_failure
}

@test "trust context - trust context with an online authority; Credential Exchange is performed" {
  auth_port="$(random_port)"
  node_port="$(random_port)"
  $OCKAM identity create alice
  $OCKAM identity create bob
  $OCKAM identity create attacker
  $OCKAM identity create authority
  bob_id=$($OCKAM identity show bob)
  alice_id=$($OCKAM identity show alice)
  authority_identity=$($OCKAM identity show --full --encoding hex authority)

  trusted="{\"$bob_id\": {}, \"$alice_id\": {}}"
  run "$OCKAM" authority create --identity authority --tcp-listener-address="127.0.0.1:$auth_port" --project-identifier "test-context" --trusted-identities "$trusted"
  assert_success
  sleep 1

  echo "{\"id\": \"test-context\",
        \"authority\" : {
            \"identity\" : \"$authority_identity\",
            \"own_credential\" :{
                \"FromCredentialIssuer\" : {
                    \"identity\": \"$authority_identity\",
                    \"multiaddr\" : \"/dnsaddr/127.0.0.1/tcp/$auth_port/service/api\" }}}}" >"$OCKAM_HOME/trust_context.json"

  run "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$node_port --trust-context "$OCKAM_HOME/trust_context.json"
  assert_success
  sleep 1

  # send a message to alice using the trust context
  msg=$(random_str)
  run "$OCKAM" message send --identity bob --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context "$OCKAM_HOME/trust_context.json" $msg
  assert_success
  assert_output "$msg"

  # send a message to authority node echo service to make sure we can use it as a healthcheck endpoint
  run "$OCKAM" message send --timeout 2 --identity bob --to "/dnsaddr/127.0.0.1/tcp/$auth_port/secure/api/service/echo" $msg
  assert_success
  assert_output "$msg"

  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context "$OCKAM_HOME/trust_context.json" $msg
  assert_failure
  run "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context $msg
  assert_failure
}

@test "trust context - trust context with an id and authority using orchestrator; orchestrator enrollment and connection is performed, orchestrator" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  $OCKAM trust-context create orchestrator-test

  run "$OCKAM" identity create m1
  $OCKAM project ticket >"$OCKAM_HOME/m1.token"
  run "$OCKAM" project enroll $OCKAM_HOME/m1.token --identity m1

  run "$OCKAM" identity create m2
  $OCKAM project ticket >"$OCKAM_HOME/m2.token"
  run "$OCKAM" project enroll $OCKAM_HOME/m2.token --identity m2

  run "$OCKAM" node create n1 --identity m1 --trust-context orchestrator-test
  assert_success

  run "$OCKAM" node create n2 --identity m2 --trust-context orchestrator-test
  assert_success

  run bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
  assert_success
}
