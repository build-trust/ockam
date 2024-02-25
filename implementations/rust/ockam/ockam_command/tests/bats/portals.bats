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

@test "portals - create tcp outlet on implicit default node" {
  run_success "$OCKAM" node delete --all -y

  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"
}

@test "portals - create tcp outlet" {
  run_success "$OCKAM" node delete --all -y

  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port" --from "test-outlet"
  assert_output --partial "/service/test-outlet"

  # The first outlet that is created without `--from` flag should be named `outlet`
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"

  # After that, the next outlet should be randomly named
  run_success $OCKAM tcp-outlet create --to "127.0.0.1:$outlet_port"
  refute_output --partial "/service/outlet"
}

@test "portals - tcp inlet CRUD" {
  outlet_port="$(random_port)"
  inlet_port="$(random_port)"

  # Create nodes for inlet/outlet pair
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # Create inlet/outlet pair
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$inlet_port --to /node/n1/service/outlet --alias "test-inlet"
  run_success $OCKAM tcp-inlet create --at /node/n2 --from 6102 --to /node/n1/service/outlet

  sleep 1

  # Check that inlet is available for deletion and delete it
  run_success $OCKAM tcp-inlet show test-inlet --at /node/n2 --output json
  assert_output --partial "\"alias\":\"test-inlet\""
  assert_output --partial "\"bind_addr\":\"127.0.0.1:$inlet_port\""

  run_success $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes

  # Test deletion of a previously deleted TCP inlet
  run_failure $OCKAM tcp-inlet delete "test-inlet" --at /node/n2 --yes
  assert_output --partial "not found"
}

@test "portals - tcp outlet CRUD" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  only_port="$(random_port)"
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet create --at /node/n2 --to $only_port

  run_success $OCKAM tcp-outlet show outlet --at /node/n1
  assert_output --partial "\"worker_addr\":\"/service/outlet\""
  assert_output --partial "\"socket_addr\":\"127.0.0.1:$port\""

  run_success $OCKAM tcp-outlet delete "outlet" --yes

  # Test deletion of a previously deleted TCP outlet
  run_success $OCKAM tcp-outlet delete "outlet" --yes
  assert_output --partial "[]"
}

@test "portals - list inlets on a node" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias tcp-inlet-2
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list outlets on a node" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet list --at /node/n1
  assert_output --partial "/service/outlet"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - show a tcp inlet" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias "test-inlet"
  sleep 1

  run_success $OCKAM tcp-inlet show "test-inlet" --at /node/n2

  # Test if non-existing TCP inlet returns NotFound
  run_failure $OCKAM tcp-inlet show "non-existing-inlet"
  assert_output --partial "not found"
}

@test "portals - show a tcp outlet" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1

  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet show "outlet"

  # Test if non-existing TCP outlet returns NotFound
  run_failure $OCKAM tcp-outlet show "non-existing-outlet"
  assert_output --partial "not found"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it" {
  port="$(random_port)"
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "127.0.0.1:$port" --to /node/n1/service/outlet

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - create an inlet/outlet pair with relay through a relay and move tcp traffic through it" {
  port="$(random_port)"
  run_success "$OCKAM" node create relay
  run_success "$OCKAM" node create blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create blue --at /node/relay --to /node/blue

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"

  run_success "$OCKAM" secure-channel list --at green
  assert_output --partial "/service"
}

@test "portals - fail to create two TCP outlets with the same address" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  o="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --to "127.0.0.1:$port" --from "$o"

  port="$(random_port)"
  run_failure "$OCKAM" tcp-outlet create --at "$n" --to "127.0.0.1:$port" --from "$o"
}

@test "portals - fail to create two TCP inlets with the same alias" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --to "127.0.0.1:$port"

  i="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"

  port="$(random_port)"
  run_failure "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet" --alias "$i"
}

@test "portals - fail to create two TCP inlets at the same address" {
  n="$(random_str)"
  run_success "$OCKAM" node create "$n"

  o="$(random_str)"
  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at "$n" --to "127.0.0.1:$port" --from "$o"

  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"

  run_failure "$OCKAM" tcp-inlet create --at "$n" --from "127.0.0.1:$port" --to "/node/$n/service/outlet"
}

@test "portals - local inlet and outlet, removing and re-creating the outlet" {
  port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to /node/blue/secure/api/service/outlet
  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"

  run_success "$OCKAM" node delete blue --yes
  run_failure curl --fail --head --max-time 10 "127.0.0.1:$port"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  sleep 20
  run_success curl --head --retry-connrefused --retry 2 --max-time 10 "127.0.0.1:$port"
}

@test "portals - local inlet and outlet in reverse order" {
  inlet_port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" node create n1
  run_success "$OCKAM" tcp-inlet create --at /node/n1 --from "127.0.0.1:${inlet_port}" --to "/ip4/127.0.0.1/tcp/${node_port}/service/outlet"

  run_success "$OCKAM" node create n2 --tcp-listener-address "127.0.0.1:${node_port}"
  run_success "$OCKAM" tcp-outlet create --at /node/n2 --to 127.0.0.1:5000

  sleep 15

  run_success curl --fail --head --retry 4 --max-time 30 "127.0.0.1:${inlet_port}"
}

@test "portals - local portal, inlet credential expires" {
  inlet_port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --expect-cached-credential

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --tcp-listener-address "127.0.0.1:$node_port" --identity bob --authority-identity $authority_identity --expect-cached-credential

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to 127.0.0.1:5000
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "127.0.0.1:$inlet_port" --to /node/bob/secure/api/service/outlet

  # Downloading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./$file_name" bs=1M count=50 && popd
  # TODO: should be run_failure after we add outgoing access control
  run_success curl --max-time 30 --limit-rate 5M -S -O "http://127.0.0.1:$inlet_port/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl --max-time 30 -O "http://127.0.0.1:$inlet_port/$file_name"

  rm "$OCKAM_HOME_BASE/$file_name"
}

@test "portals - local portal, outlet credential expires" {
  inlet_port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --expect-cached-credential

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --tcp-listener-address "127.0.0.1:$node_port" --identity bob --authority-identity $authority_identity --expect-cached-credential

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to 127.0.0.1:5000
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "127.0.0.1:$inlet_port" --to /node/bob/secure/api/service/outlet

  # Downloading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./$file_name" bs=1M count=50 && popd
  run_failure curl --max-time 30 --limit-rate 5M -S -O "http://127.0.0.1:$inlet_port/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl --max-time 30 -O "http://127.0.0.1:$inlet_port/$file_name"

  rm "$OCKAM_HOME_BASE/$file_name"
}
