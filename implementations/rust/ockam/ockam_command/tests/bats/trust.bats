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

@test "trust - no trust; everything is accepted" {
  run_success "$OCKAM" identity create m1
  run_success "$OCKAM" node create n1 --identity m1

  run_success "$OCKAM" identity create m2
  run_success "$OCKAM" node create n2 --identity m2

  run_success bash -c "$OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api \
        | $OCKAM message send hello --from /node/n1 --to -/service/echo"
}

@test "trust - offline authority; Credential Exchange is performed" {
  port="$(random_port)"

  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  run_success "$OCKAM" identity create attacker
  attacker_identifier=$($OCKAM identity show attacker)
  attacker_identity=$($OCKAM identity show attacker --full --encoding hex)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --tcp-listener-address "127.0.0.1:$port" --identity alice --authority-identity $authority_identity --expect-cached-credential

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --identity bob --authority-identity $authority_identity --expect-cached-credential

  # issue and store credentials for alice
  $OCKAM credential issue --as authority --for "$alice_identifier" --attribute city="New York" --encoding hex >"$OCKAM_HOME/alice.cred"
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential-path "$OCKAM_HOME/alice.cred"

  # issue and store credential for bob
  $OCKAM credential issue --as authority --for "$bob_identifier" --attribute city="New York" --encoding hex >"$OCKAM_HOME/bob.cred"
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential-path "$OCKAM_HOME/bob.cred"

  msg=$(random_str)

  # Create a node for attacker
  run_success "$OCKAM" node create attacker --identity attacker --authority-identity attacker_identity

  # Fail, attacker won't present any credential
  run_failure $OCKAM message send --timeout 2 --from attacker --to "/dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo" $msg

  # Fail, attacker will present an invalid credential (self signed rather than signed by authority)
  $OCKAM credential issue --as attacker --for $attacker_identifier --encoding hex >"$OCKAM_HOME/attacker.cred"
  run_failure "$OCKAM" credential store --node attacker --issuer "$attacker_identity" --credential-path "$OCKAM_HOME/attacker.cred"

  run_success $OCKAM message send --timeout 2 --from bob --to "/dnsaddr/127.0.0.1/tcp/$port/secure/api/service/echo" $msg
  assert_output "$msg"
}

@test "trust - online authority; Credential Exchange is performed" {
  auth_port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  run_success "$OCKAM" identity create attacker
  attacker_identifier=$($OCKAM identity show attacker)
  attacker_identity=$($OCKAM identity show attacker --full --encoding hex)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  trusted="{\"$bob_identifier\": {}}"
  run_success "$OCKAM" authority create --identity authority --tcp-listener-address="127.0.0.1:$auth_port" --project-identifier test --trusted-identities "$trusted"
  assert_success
  sleep 1

  authority_route="/dnsaddr/127.0.0.1/tcp/$auth_port/service/api"
  run_success "$OCKAM" node create --identity alice --tcp-listener-address 127.0.0.1:$node_port --authority-identity $authority_identity
  sleep 1

  run_success "$OCKAM" node create bob_node --identity bob --authority-identity $authority_identity --authority-route $authority_route
  sleep 1

  # send a message to alice using the trust context
  msg=$(random_str)
  run_success "$OCKAM" message send --timeout 2 --identity bob --from bob_node --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo $msg
  assert_output "$msg"

  # send a message to authority node echo service to make sure we can use it as a healthcheck endpoint
  run_success "$OCKAM" message send --timeout 2 --identity bob --to "/dnsaddr/127.0.0.1/tcp/$auth_port/secure/api/service/echo" $msg
  assert_output "$msg"

  run_failure "$OCKAM" message send --timeout 2 --identity attacker --to /dnsaddr/127.0.0.1/tcp/$node_port/secure/api/service/echo $msg
}
