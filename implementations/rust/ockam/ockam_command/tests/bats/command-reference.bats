#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
}

setup_orchestrator_test() {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS https://docs.ockam.io/reference/command/nodes
@test "nodes" {
  run_success "$OCKAM" node create

  n1="$(random_str)"
  n2="$(random_str)"
  run_success "$OCKAM" node create "$n1"
  run_success "$OCKAM" node create "$n2" --verbose

  run_success "$OCKAM" node list
  assert_output --partial "\"node_name\": \"$n1\""
  assert_output --partial "\"status\": \"Running\""

  run_success "$OCKAM" node stop "$n1"
  assert_output --partial "Stopped node '$n1'"

  run_success "$OCKAM" node start "$n1"

  run "$OCKAM" node delete "$n1" --yes
  run "$OCKAM" node delete --all --yes
}

@test "workers and services" {
  n1="$(random_str)"
  run_success "$OCKAM" node create "$n1"
  run_success "$OCKAM" worker list --at "$n1"
  run_success "$OCKAM" message send hello --to "/node/$n1/service/uppercase"
  assert_output "HELLO"
}

@test "projects - list" {
  setup_orchestrator_test

  run_success "$OCKAM" project list
}

@test "space - list" {
  setup_orchestrator_test

  run_success "$OCKAM" space list
}

# ===== TESTS https://docs.ockam.io/reference/command/routing
@test "routing" {
  n1="$(random_str)"

  run_success "$OCKAM" node create "$n1"

  run_success "$OCKAM" message send 'Hello Ockam!' --to "/node/$n1/service/echo"
  assert_output "Hello Ockam!"

  run_success "$OCKAM" service start hop --addr h1
  run_success "$OCKAM" message send hello --to "/node/$n1/service/h1/service/echo"
  assert_output "hello"

  run_success "$OCKAM" service start hop --addr h2

  run_success "$OCKAM" message send hello --to "/node/$n1/service/h1/service/h2/service/echo"
  assert_output "hello"
}

@test "transports" {
  n1="$(random_str)"
  n2="$(random_str)"
  n2_port="$(random_port)"
  n3="$(random_str)"
  n3_port="$(random_port)"

  run_success "$OCKAM" node create "$n1"
  run_success "$OCKAM" node create "$n2" --tcp-listener-address="127.0.0.1:$n2_port"
  run_success "$OCKAM" node create "$n3" --tcp-listener-address="127.0.0.1:$n3_port"
  run_success "$OCKAM" service start hop --at "$n2"

  n1_id=$("$OCKAM" tcp-connection create --from "$n1" --to "127.0.0.1:$n2_port" | grep -o "[0-9a-f]\{32\}" | head -1)
  n2_id=$("$OCKAM" tcp-connection create --from "$n2" --to "127.0.0.1:$n3_port" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from "$n1" --to /worker/${n1_id}/service/hop/worker/${n2_id}/service/uppercase
  assert_output "HELLO"
}

# ===== TESTS https://docs.ockam.io/reference/command/advanced-routing
@test "relays and portals" {
  n1="$(random_str)"
  n2="$(random_str)"
  n2_port="$(random_port)"
  n3="$(random_str)"
  inlet_port="$(random_port)"

  run_success "$OCKAM" node create "$n2" --tcp-listener-address="127.0.0.1:$n2_port"
  run_success "$OCKAM" node create "$n3"
  run_success "$OCKAM" service start hop --at "$n3"

  run_success "$OCKAM" relay create "$n3" --at "/node/$n2" --to "/node/$n3"
  run_success "$OCKAM" node create "$n1"

  n1_id=$("$OCKAM" tcp-connection create --from "$n1" --to "127.0.0.1:$n2_port" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from "$n1" --to "/worker/${n1_id}/service/forward_to_$n3/service/uppercase"
  assert_output "HELLO"

  run_success "$OCKAM" tcp-outlet create --at "$n3" --from /service/outlet --to "127.0.0.1:5000"
  run_success "$OCKAM" tcp-inlet create --at "$n1" --from "127.0.0.1:$inlet_port" --to "/worker/${n1_id}/service/forward_to_$n3/service/hop/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$inlet_port"
}

# ===== TESTS https://docs.ockam.io/reference/command/routing
@test "vaults and identities" {
  v1="$(random_str)"
  i1="$(random_str)"
  run_success "$OCKAM" vault create "$v1"
  run_success "$OCKAM" identity create "$i1" --vault "$v1"
  run_success "$OCKAM" identity show "$i1"
  run_success "$OCKAM" identity show "$i1" --full
}

# ===== TESTS https://docs.ockam.io/reference/command/secure-channels
@test "identifiers" {
  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" node create "$a"
  run_success "$OCKAM" node create "$b"

  id=$("$OCKAM" secure-channel create --from "$a" --to "/node/$b/service/api" | grep -o "[0-9a-f]\{32\}" | head -1)

  run_success "$OCKAM" message send hello --from "$a" --to /service/${id}/service/uppercase
  assert_output "HELLO"

  "$OCKAM" secure-channel create --from "$a" --to "/node/$b/service/api" |
    "$OCKAM" message send hello --from "$a" --to -/service/uppercase

  output=$("$OCKAM" secure-channel create --from "$a" --to "/node/$b/service/api" |
    "$OCKAM" message send hello --from "$a" --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "through relays" {
  relay="$(random_str)"
  a="$(random_str)"
  b="$(random_str)"
  port="$(random_port)"

  run_success "$OCKAM" node create "$relay" --tcp-listener-address="127.0.0.1:$port"
  run_success "$OCKAM" node create "$b"
  run_success "$OCKAM" relay create "$b" --at "/node/$relay" --to "$b"
  run_success "$OCKAM" node create "$a"

  worker_id=$("$OCKAM" tcp-connection create --from "$a" --to "127.0.0.1:$port" | grep -o "[0-9a-f]\{32\}" | head -1)

  output=$("$OCKAM" secure-channel create --from "$a" --to "/worker/${worker_id}/service/forward_to_$b/service/api" |
    "$OCKAM" message send hello --from "$a" --to -/service/uppercase)
  assert [ "$output" == "HELLO" ]
}

@test "elastic encrypted relays" {
  setup_orchestrator_test

  "$OCKAM" project information --output json >/${BATS_TEST_TMPDIR}/project.json

  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" node create "$a" --project-path /${BATS_TEST_TMPDIR}/project.json
  run_success "$OCKAM" node create "$b" --project-path /${BATS_TEST_TMPDIR}/project.json
  run_success "$OCKAM" relay create "$b" --at /project/default --to "/node/$a"

  output=$("$OCKAM" secure-channel create --from "$a" --to "/project/default/service/forward_to_$b/service/api" |
    "$OCKAM" message send hello --from "$a" --to -/service/uppercase)
  assert [ "$output" == "HELLO" ]
}

# ===== TESTS https://docs.ockam.io/reference/command/credentials
@test "issuing credentials" {
  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" identity create "$a"
  run_success "$OCKAM" identity create "$b"

  id=$("$OCKAM" identity show "$b")

  run_success "$OCKAM" credential issue --as "$a" --for ${id}
  run_success "$OCKAM" credential issue --as "$a" --for ${id} --attribute location=Chicago --attribute department=Operations
}

@test "verifying - storing credentials" {
  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" identity create "$a"
  run_success "$OCKAM" identity create "$b"

  id_a=$("$OCKAM" identity show "$a" --full --encoding hex)
  id_a_short=$("$OCKAM" identity show "$a")
  id_b_short=$("$OCKAM" identity show "$b")

  "$OCKAM" credential issue --as "$a" --for ${id_b_short} --encoding hex >/${BATS_TEST_TMPDIR}/b.credential

  run_success "$OCKAM" credential verify --issuer ${id_a_short} --credential-path /${BATS_TEST_TMPDIR}/b.credential
  run_success "$OCKAM" credential store c1 --issuer ${id_a} --credential-path /${BATS_TEST_TMPDIR}/b.credential
}

@test "trust anchors" {
  n1="$(random_str)"
  n2="$(random_str)"
  i1="$(random_str)"
  i2="$(random_str)"

  run_success "$OCKAM" identity create "$i1"

  "$OCKAM" identity show "$i1" >/${BATS_TEST_TMPDIR}/i1.identifier

  run_success "$OCKAM" node create "$n1" --identity "$i1"
  run_success "$OCKAM" identity create "$i2"

  "$OCKAM" identity show "$i2" >/${BATS_TEST_TMPDIR}/i2.identifier

  run_success "$OCKAM" node create "$n2" --identity "$i2"
  run_success "$OCKAM" secure-channel-listener create l --at "$n2" \
    --identity "$i2" --authorized $(cat /${BATS_TEST_TMPDIR}/i1.identifier)

  output=$("$OCKAM" secure-channel create \
    --from "$n1" --to /node/n2/service/l \
    --identity "$i1" --authorized $(cat /${BATS_TEST_TMPDIR}/i2.identifier) |
    "$OCKAM" message send hello --from "$n1" --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "anchoring trust in a credential issuer" {
  n1="$(random_str)"
  n2="$(random_str)"
  i1="$(random_str)"
  i2="$(random_str)"
  authority="$(random_str)"
  authority_exported=/${BATS_TEST_TMPDIR}/authority

  run_success "$OCKAM" identity create "$authority"

  "$OCKAM" identity show "$authority" >/${BATS_TEST_TMPDIR}/authority.identifier
  "$OCKAM" identity show "$authority" --full --encoding hex >$authority_exported

  run_success "$OCKAM" identity create "$i1"

  "$OCKAM" identity show "$i1" >/${BATS_TEST_TMPDIR}/i1
  "$OCKAM" credential issue --as "$authority" --for $(cat /${BATS_TEST_TMPDIR}/i1) --attribute city="New York" --encoding hex >/${BATS_TEST_TMPDIR}/i1.credential

  run_success "$OCKAM" credential store c1 --issuer $(cat $authority_exported) --credential-path /${BATS_TEST_TMPDIR}/i1.credential
  run_success "$OCKAM" identity create "$i2"

  "$OCKAM" identity show "$i2" >/${BATS_TEST_TMPDIR}/i2
  "$OCKAM" credential issue --as "$authority" \
    --for $(cat /${BATS_TEST_TMPDIR}/i2) --attribute city="San Francisco" \
    --encoding hex >/${BATS_TEST_TMPDIR}/i2.credential

  run_success "$OCKAM" credential store c2 --issuer $(cat $authority_exported) --credential-path /${BATS_TEST_TMPDIR}/i2.credential
  run_success "$OCKAM" node create "$n1" --identity "$i1" --authority-identity $(cat $authority_exported)
  run_success "$OCKAM" node create "$n2" --identity "$i2" --authority-identity $(cat $authority_exported) --credential c2

  output=$("$OCKAM" secure-channel create --from "$n1" --to /node/n2/service/api --credential c1 --identity "$i1" |
    "$OCKAM" message send hello --from "$n1" --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "managed authorities" {
  setup_orchestrator_test

  "$OCKAM" project information --output json >/${BATS_TEST_TMPDIR}/project.json

  a="$(random_str)"
  b="$(random_str)"

  run_success "$OCKAM" node create "$a" --project-path /${BATS_TEST_TMPDIR}/project.json
  run_success "$OCKAM" node create "$b" --project-path /${BATS_TEST_TMPDIR}/project.json

  run_success "$OCKAM" relay create "$b" --at /project/default --to "/node/$a/service/forward_to_$b"

  output=$("$OCKAM" secure-channel create --from "$a" --to "/project/default/service/forward_to_$b/service/api" |
    "$OCKAM" message send hello --from "$a" --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}
