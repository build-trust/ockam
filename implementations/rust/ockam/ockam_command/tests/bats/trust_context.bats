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
  run_success "$OCKAM" trust-context create

  # Create with specific name
  t=$(random_str)
  run_success "$OCKAM" trust-context create "${t}"

  # List
  run_success "$OCKAM" trust-context list
  assert_output --partial "${t}"

  # Change the default
  run_success "$OCKAM" trust-context default "${t}"
  run_success "$OCKAM" trust-context show
  assert_output --partial "${t}"

  # Delete and verify
  run_success "$OCKAM" trust-context delete "${t}" --yes
  run_failure "$OCKAM" trust-context show "${t}"
}

@test "trust context - no trust context; everything is accepted" {
  run_success "$OCKAM" identity create m1
  run_success "$OCKAM" node create n1 --identity m1

  run_success "$OCKAM" identity create m2
  run_success "$OCKAM" node create n2 --identity m2

  run_success bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
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
  run_success "$OCKAM" authority create --identity authority --tcp-listener-address="127.0.0.1:$auth_port" --project-identifier "test-context" --trusted-identities "$trusted"
  assert_success
  sleep 1

  echo "{\"id\": \"test-context\",
        \"authority\" : {
            \"identity\" : \"$authority_identity\",
            \"own_credential\" :{
                \"FromCredentialIssuer\" : {
                    \"identity\": \"$authority_identity\",
                    \"multiaddr\" : \"/dnsaddr/127.0.0.1/tcp/$auth_port/service/api\" }}}}" >"$OCKAM_HOME/trust_context.json"

  run_success "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$node_port --trust-context "$OCKAM_HOME/trust_context.json"
  sleep 1

  # send a message to alice using the trust context
  msg=$(random_str)
  run_success "$OCKAM" message send --identity bob --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context "$OCKAM_HOME/trust_context.json" $msg
  assert_output "$msg"

  # send a message to authority node echo service to make sure we can use it as a healthcheck endpoint
  run_success "$OCKAM" message send --timeout 2 --identity bob --to "/dnsaddr/127.0.0.1/tcp/$auth_port/secure/api/service/echo" $msg
  assert_output "$msg"

  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context "$OCKAM_HOME/trust_context.json" $msg
  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context $msg
}
