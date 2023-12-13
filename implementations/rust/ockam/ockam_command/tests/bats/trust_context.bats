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

@test "trust context - trust context with an id only; ABAC rules are applied" {
  run_success "$OCKAM" identity create m1

  m1_identifier=$(run_success "$OCKAM" identity show m1)
  trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\", \"project_id\" : \"1\", \"trust_context_id\" : \"1\"}}"

  run_success "$OCKAM" node create n1 --identity m1
  run_success "$OCKAM" trust-context create default --id 1
  run_success "$OCKAM" node create n2 --trust-context default --trusted-identities "$trusted"
  run_success bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
  run_failure "$OCKAM" message send hello --timeout 2 --from /node/n1 --to /node/n2/service/echo
}

@test "trust context - trust context with an offline authority; Credential Exchange is performed" {
  port="$(random_port)"
  # Create two identities
  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  run_success "$OCKAM" identity create attacker
  attacker_identifier=$($OCKAM identity show attacker)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # issue and store credentials for alice
  $OCKAM credential issue --as authority --for $alice_identifier --attribute city="New York" --encoding hex >"$OCKAM_HOME/alice.cred"
  run_success "$OCKAM" credential store alice-cred --issuer "$authority_identity" --credential-path "$OCKAM_HOME/alice.cred"
  run_success "$OCKAM" trust-context create alice-trust-context --credential alice-cred

  # issue and store credential for bob
  $OCKAM credential issue --as authority --for "$bob_identifier" --attribute city="New York" --encoding hex >"$OCKAM_HOME/bob.cred"
  run_success "$OCKAM" credential store bob-cred --issuer "$authority_identity" --credential-path "$OCKAM_HOME/bob.cred"
  run_success "$OCKAM" trust-context create bob-trust-context --credential bob-cred

  # Create a node for alice that trust authority as a credential authority
  run_success "$OCKAM" node create alice --tcp-listener-address "127.0.0.1:$port" --identity alice --trust-context alice-trust-context

  msg=$(random_str)

  # Fail, attacker won't present any credential
  run_failure $OCKAM message send --timeout 2 --identity attacker --to "/dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo" $msg

  # Fail, attacker will present an invalid credential (self signed rather than signed by authority)
  $OCKAM credential issue --as attacker --for $attacker_identifier --encoding hex >"$OCKAM_HOME/attacker.cred"
  run_failure "$OCKAM" credential store att-cred --issuer "$authority_identity" --credential-path "$OCKAM_HOME/attacker.cred"

  # Fail, attacker will present an invalid credential (bob's credential, not own)
  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg

  run_success "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg
  assert_output $msg

  run_success "$OCKAM" node delete alice --yes
  run_success "$OCKAM" trust-context create alice-trust-context --id "$authority_id"

  run_success "$OCKAM" node create alice --tcp-listener-address 127.0.0.1:$port --identity alice --trust-context alice-trust-context

  run_failure "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo --trust-context bob-trust-context $msg
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
  run_success "$OCKAM" authority create --identity authority --tcp-listener-address="127.0.0.1:$auth_port" --project-identifier test-context --trusted-identities "$trusted"
  assert_success
  sleep 1

  authority_route="/dnsaddr/127.0.0.1/tcp/$auth_port/service/api"
  run_success "$OCKAM" trust-context create test-context --id test-context --authority-identity $authority_identity --authority-route $authority_route
  run_success "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$node_port --trust-context test-context
  sleep 1

  # send a message to alice using the trust context
  msg=$(random_str)
  run_success "$OCKAM" message send --timeout 2 --identity bob --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context test-context $msg
  assert_output "$msg"

  # send a message to authority node echo service to make sure we can use it as a healthcheck endpoint
  run_success "$OCKAM" message send --timeout 2 --identity bob --to "/dnsaddr/127.0.0.1/tcp/$auth_port/secure/api/service/echo" $msg
  assert_output "$msg"

  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo --trust-context test-context $msg
  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo $msg
}
