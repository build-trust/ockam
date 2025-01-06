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

# ===== TESTS https://docs.ockam.io/reference/command/nodes

@test "nodes" {
  run_success "$OCKAM" node create

  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2 --verbose

  run_success "$OCKAM" node list
  assert_output --partial "\"node_name\": \"n1\""
  assert_output --partial "\"status\": \"running\""

  run_success "$OCKAM" node stop n1
  assert_output --partial "n1 was stopped"

  run_success "$OCKAM" node start n1

  run_success "$OCKAM" node delete n1 --yes
  run_success "$OCKAM" node delete --all --yes
}

@test "workers and services" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" worker list --at n1
  run_success "$OCKAM" message send hello --to "/node/n1/service/uppercase"
  assert_output "HELLO"
}

# ===== TESTS https://docs.ockam.io/reference/command/routing
@test "routing" {
  run_success "$OCKAM" node create n1

  run_success "$OCKAM" message send 'Hello Ockam!' --to "/node/n1/service/echo"
  assert_output "Hello Ockam!"

  run_success "$OCKAM" service start hop --addr h1
  run_success "$OCKAM" message send hello --to "/node/n1/service/h1/service/echo"
  assert_output "hello"

  run_success "$OCKAM" service start hop --addr h2

  run_success "$OCKAM" message send hello --to "/node/n1/service/h1/service/h2/service/echo"
  assert_output "hello"
}

@test "transports" {
  n2_port="$(random_port)"
  n3_port="$(random_port)"

  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2 --tcp-listener-address="127.0.0.1:$n2_port"
  run_success "$OCKAM" node create n3 --tcp-listener-address="127.0.0.1:$n3_port"
  run_success "$OCKAM" service start hop --at n2

  n1_id=$("$OCKAM" tcp-connection create --from n1 --to "127.0.0.1:$n2_port" | grep -o "[0-9a-f]\{32\}" | head -1)
  n2_id=$("$OCKAM" tcp-connection create --from n2 --to "127.0.0.1:$n3_port" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from n1 --to /worker/${n1_id}/service/hop/worker/${n2_id}/service/uppercase
  assert_output "HELLO"
}

# ===== TESTS https://docs.ockam.io/reference/command/advanced-routing
@test "relays and portals" {
  n2_port="$(random_port)"
  inlet_port="$(random_port)"

  run_success "$OCKAM" node create n2 --tcp-listener-address="127.0.0.1:$n2_port"
  run_success "$OCKAM" node create n3
  run_success "$OCKAM" service start hop --at n3

  run_success "$OCKAM" relay create n3 --at "/node/n2" --to "/node/n3"
  run_success "$OCKAM" node create n1

  n1_id=$("$OCKAM" tcp-connection create --from n1 --to "127.0.0.1:$n2_port" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from n1 --to "/worker/${n1_id}/service/forward_to_n3/service/uppercase"
  assert_output "HELLO"

  run_success "$OCKAM" tcp-outlet create --at n3 --to "$PYTHON_SERVER_PORT"
  run_success "$OCKAM" tcp-inlet create --at n1 --from "$inlet_port" --to "/worker/${n1_id}/service/forward_to_n3/service/hop/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}

# ===== TESTS https://docs.ockam.io/reference/command/routing
@test "vaults and identities" {
  run_success "$OCKAM" vault create v1
  run_success "$OCKAM" identity create i1 --vault v1
  run_success "$OCKAM" identity show i1
  run_success "$OCKAM" identity show i1 --full
}

# ===== TESTS https://docs.ockam.io/reference/command/secure-channels
@test "identifiers" {
  run_success "$OCKAM" node create a
  run_success "$OCKAM" node create b

  id=$("$OCKAM" secure-channel create --from a --to /node/b/service/api | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from a --to "/service/${id}/service/uppercase"
  assert_output "HELLO"

  "$OCKAM" secure-channel create --from a --to /node/b/service/api |
    "$OCKAM" message send hello --from a --to -/service/uppercase

  run_success bash -c "$OCKAM secure-channel create --from a --to /node/b/service/api |
    $OCKAM message send hello --from a --to -/service/uppercase"
  assert_output "HELLO"

}

@test "through relays" {
  relay="$(random_str)"
  port="$(random_port)"

  run_success "$OCKAM" node create "$relay" --tcp-listener-address="127.0.0.1:$port"
  run_success "$OCKAM" node create b
  run_success "$OCKAM" relay create b --at "/node/$relay" --to b
  run_success "$OCKAM" node create a

  worker_id=$("$OCKAM" tcp-connection create --from a --to "127.0.0.1:$port" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success bash -c "$OCKAM secure-channel create --from a --to /worker/${worker_id}/service/forward_to_b/service/api |
    $OCKAM message send hello --from a --to -/service/uppercase"
  assert_output "HELLO"
}

# ===== TESTS https://docs.ockam.io/reference/command/credentials
@test "issuing credentials" {
  run_success "$OCKAM" identity create a
  run_success "$OCKAM" identity create b

  id=$("$OCKAM" identity show b)

  run_success "$OCKAM" credential issue --as a --for ${id}
  run_success "$OCKAM" credential issue --as a --for ${id} --attribute location=Chicago --attribute department=Operations
}

@test "verifying - storing credentials" {
  run_success "$OCKAM" identity create a
  run_success "$OCKAM" identity create b

  id_a=$("$OCKAM" identity show a --full --encoding hex)
  id_a_short=$("$OCKAM" identity show a)
  id_b_short=$("$OCKAM" identity show b)

  "$OCKAM" credential issue --as a --for ${id_b_short} --encoding hex >/${BATS_TEST_TMPDIR}/b.credential

  run_success "$OCKAM" credential verify --issuer ${id_a_short} --credential-path /${BATS_TEST_TMPDIR}/b.credential
}

@test "trust anchors" {
  run_success "$OCKAM" identity create i1

  "$OCKAM" identity show i1 >/${BATS_TEST_TMPDIR}/i1.identifier

  run_success "$OCKAM" node create n1 --identity i1
  run_success "$OCKAM" identity create i2

  "$OCKAM" identity show i2 >/${BATS_TEST_TMPDIR}/i2.identifier

  run_success "$OCKAM" node create n2 --identity i2
  run_success "$OCKAM" secure-channel-listener create l --at n2 \
    --identity i2 --authorized $(cat /${BATS_TEST_TMPDIR}/i1.identifier)

  run_success bash -c "$OCKAM secure-channel create \
    --from n1 --to /node/n2/service/l \
    --identity i1 --authorized $(cat /${BATS_TEST_TMPDIR}/i2.identifier) |
    $OCKAM message send hello --from n1 --to -/service/uppercase"
  assert_output "HELLO"
}

@test "anchoring trust in a credential issuer" {
  run_success "$OCKAM" identity create authority
  AUTHORITY_IDENTIFIER=$("$OCKAM" identity show authority)
  AUTHORITY_IDENTITY=$("$OCKAM" identity show authority --full --encoding hex)

  run_success "$OCKAM" identity create i1
  run_success "$OCKAM" identity create i2
  I1_IDENTIFIER=$("$OCKAM" identity show i1)
  I1_CREDENTIAL=$("$OCKAM" credential issue --as authority \
    --for "$I1_IDENTIFIER" --attribute city="New York" \
    --encoding hex)

  run_success "$OCKAM" node create n1 --identity i1 --authority-identity "$AUTHORITY_IDENTITY" --credential-scope "test"
  run_success "$OCKAM" node create n2 --identity i2 --authority-identity "$AUTHORITY_IDENTITY"

  run_success "$OCKAM" credential store --issuer "$AUTHORITY_IDENTIFIER" --credential "$I1_CREDENTIAL" --at n1 --scope "test"

  run_success bash -c "$OCKAM secure-channel create --from n1 --identity i1 --to /node/n2/service/api |
    $OCKAM message send --timeout 1 hello --from n1 --to -/service/echo"
  assert_output "hello"

  run_failure bash -c "$OCKAM secure-channel create --from n2 --identity i2 --to /node/n1/service/api |
    $OCKAM message send --timeout 1 hello --from n2 --to -/service/echo"
}
