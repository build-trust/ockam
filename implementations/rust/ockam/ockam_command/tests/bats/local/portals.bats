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

  # Create nodes for inlet/outlet pair
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # Create inlet/outlet pair
  outlet_port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$outlet_port"
  assert_output --partial "/service/outlet"

  inlet_port="$(random_port)"
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
  run_success "$OCKAM" node create n1

  run_success "$OCKAM" node create n2

  port_1="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port_1"
  assert_output --partial "/service/outlet"

  port_2="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n2 --to $port_2

  run_success $OCKAM tcp-outlet show outlet --at /node/n1
  assert_output --partial "\"worker_addr\":\"/service/outlet\""
  assert_output --partial "\"to\":\"127.0.0.1:$port_1\""

  run_success $OCKAM tcp-outlet delete "outlet" --yes

  # Test deletion of a previously deleted TCP outlet
  run_success $OCKAM tcp-outlet delete "outlet" --yes
  assert_output --partial "[]"
}

@test "portals - list inlets on a node" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create --at /node/n2 --from $port --to /node/n1/service/outlet --alias tcp-inlet-2
  sleep 1

  run_success $OCKAM tcp-inlet list --at /node/n2
  assert_output --partial "tcp-inlet-2"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - list outlets on a node" {
  run_success "$OCKAM" node create n1

  port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet list --at /node/n1
  assert_output --partial "/service/outlet"
  assert_output --partial "127.0.0.1:$port"
}

@test "portals - show a tcp inlet" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create --at /node/n2 --from $port --to /node/n1/service/outlet --alias "test-inlet"
  sleep 1

  run_success $OCKAM tcp-inlet show "test-inlet" --at /node/n2

  # Test if non-existing TCP inlet returns NotFound
  run_failure $OCKAM tcp-inlet show "non-existing-inlet"
  assert_output --partial "not found"
}

@test "portals - show a tcp outlet" {
  run_success "$OCKAM" node create n1

  port="$(random_port)"
  run_success $OCKAM tcp-outlet create --at /node/n1 --to "$port"
  assert_output --partial "/service/outlet"

  run_success $OCKAM tcp-outlet show "outlet"

  # Test if non-existing TCP outlet returns NotFound
  run_failure $OCKAM tcp-outlet show "non-existing-outlet"
  assert_output --partial "not found"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - create an inlet/outlet, download file" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./.tmp/$file_name" bs=1M count=50 && popd
  run_success curl -sSf -m 20 -o "$OCKAM_HOME/$file_name" "http://127.0.0.1:$port/.tmp/$file_name"
}

@test "portals - create an inlet/outlet, upload file" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  file_name="$(random_str)".bin
  tmp_dir_name="$(random_str)"
  pushd "$OCKAM_HOME_BASE/.tmp"
  mkdir "$tmp_dir_name"
  dd if=/dev/urandom of="./$tmp_dir_name/$file_name" bs=1M count=50
  popd
  run_success curl -sS -m 20 -X POST "http://127.0.0.1:$port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it, where the outlet points to an HTTPs endpoint" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to google.com:443
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  # This test does not pass on CI
  # run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - create an inlet/outlet pair with relay through a relay and move tcp traffic through it" {
  run_success "$OCKAM" node create relay
  run_success "$OCKAM" node create blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"
  run_success "$OCKAM" relay create blue --at /node/relay --to /node/blue

  run_success "$OCKAM" node create green
  port="$(random_port)"
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"

  run_success "$OCKAM" secure-channel list --at green
  assert_output --partial "/service"
}

@test "portals - fail to create two TCP outlets with the same worker address" {
  run_success "$OCKAM" node create n

  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at n --to "$port" --from o
  port="$(random_port)"
  run_failure "$OCKAM" tcp-outlet create --at n --to "$port" --from o
}

@test "portals - fail to create two TCP inlets with the same alias" {
  run_success "$OCKAM" node create n

  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at n --to "$port"

  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at n --from "$port" --to "/node/n/service/outlet" --alias i
  port="$(random_port)"
  run_failure "$OCKAM" tcp-inlet create --at n --from "$port" --to "/node/n/service/outlet" --alias i
}

@test "portals - fail to create two TCP inlets at the same socket address" {
  run_success "$OCKAM" node create n

  port="$(random_port)"
  run_success "$OCKAM" tcp-outlet create --at n --to "$port" --from o

  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at n --from "$port" --to "/node/n/service/outlet"

  run_failure "$OCKAM" tcp-inlet create --at n --from "$port" --to "/node/n/service/outlet"
}

@test "portals - local inlet and outlet, removing and re-creating the outlet" {
  node_port="$(random_port)"
  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"

  run_success "$OCKAM" node create green
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "$inlet_port" --to /node/blue/secure/api/service/outlet
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  run_success "$OCKAM" node delete blue --yes --force
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"

  sleep 15
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}

@test "portals - local inlet and outlet in reverse order" {
  run_success "$OCKAM" node create n1
  node_port="$(random_port)"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n1 --from "${inlet_port}" --to "/ip4/127.0.0.1/tcp/${node_port}/service/outlet"

  run_success "$OCKAM" node create n2 --tcp-listener-address "127.0.0.1:${node_port}"
  run_success "$OCKAM" tcp-outlet create --at /node/n2 --to "$PYTHON_SERVER_PORT"

  sleep 15
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:${inlet_port}"
}

@test "portals - local portal, curl download, inlet credential expires" {
  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --credential-scope "test"

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --identity bob --authority-identity $authority_identity --credential-scope "test"

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential --scope "test"

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential --scope "test"

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to "$PYTHON_SERVER_PORT"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "$inlet_port" --to /node/bob/secure/api/service/outlet

  # Downloading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./.tmp/$file_name" bs=1M count=50 && popd
  run_failure curl -sSf -m 20 --limit-rate 5M \
    -o "$OCKAM_HOME/$file_name" "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl -sSf -m 20 -o "$OCKAM_HOME/$file_name" "http://127.0.0.1:$inlet_port/.tmp/$file_name"
}

@test "portals - local portal, curl upload, inlet credential expires" {
  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --credential-scope "test"

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --identity bob --authority-identity $authority_identity --credential-scope "test"

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential --scope "test"

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential --scope "test"

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to "$PYTHON_SERVER_PORT"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "$inlet_port" --to /node/bob/secure/api/service/outlet

  # Uploading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  tmp_dir_name="$(random_str)"
  pushd "$OCKAM_HOME_BASE/.tmp"
  mkdir "$tmp_dir_name"
  dd if=/dev/urandom of="./$tmp_dir_name/$file_name" bs=1M count=50
  popd
  run_failure curl -sS -m 20 --limit-rate 5M -X POST "http://127.0.0.1:$inlet_port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"

  # Consequent attempt fails
  run_failure curl -sS -m 20 -X POST "http://127.0.0.1:$inlet_port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"
}

@test "portals - local portal, curl download, outlet credential expires" {
  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --credential-scope "test"

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --identity bob --authority-identity $authority_identity --credential-scope "test"

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential --scope "test"

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential --scope "test"

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to "$PYTHON_SERVER_PORT"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "$inlet_port" --to /node/bob/secure/api/service/outlet

  # Downloading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./.tmp/$file_name" bs=1M count=50 && popd
  run_failure curl -sSf -m 20 --limit-rate 5M \
    -o "$OCKAM_HOME/$file_name" "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl -sSf -m 20 -o "$OCKAM_HOME/$file_name" "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null
}

@test "portals - local portal, curl upload, outlet credential expires" {
  run_success "$OCKAM" identity create alice
  alice_identifier=$($OCKAM identity show alice)

  run_success "$OCKAM" identity create bob
  bob_identifier=$($OCKAM identity show bob)

  # Create an identity that both alice and bob will trust
  run_success "$OCKAM" identity create authority
  authority_identifier=$($OCKAM identity show authority)
  authority_identity=$($OCKAM identity show authority --full --encoding hex)

  # Create a node for alice that trusts authority as a credential authority
  run_success "$OCKAM" node create alice --identity alice --authority-identity $authority_identity --credential-scope "test"

  # Create a node for bob that trusts authority as a credential authority
  run_success "$OCKAM" node create bob --identity bob --authority-identity $authority_identity --credential-scope "test"

  # issue and store a short-lived credential for alice
  alice_credential=$($OCKAM credential issue --as authority --for "$alice_identifier" --encoding hex)
  run_success "$OCKAM" credential store --at alice --issuer "$authority_identifier" --credential $alice_credential --scope "test"

  # issue and store credential for bob
  bob_credential=$($OCKAM credential issue --as authority --for "$bob_identifier" --ttl 5s --encoding hex)
  run_success "$OCKAM" credential store --at bob --issuer "$authority_identifier" --credential $bob_credential --scope "test"

  run_success "$OCKAM" tcp-outlet create --at /node/bob --to "$PYTHON_SERVER_PORT"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/alice --from "$inlet_port" --to /node/bob/secure/api/service/outlet

  # Uploading a file will create a long-lived TCP connection, which should be dropped by the portal
  # when the credential expires
  file_name="$(random_str)".bin
  tmp_dir_name="$(random_str)"
  pushd "$OCKAM_HOME_BASE/.tmp"
  mkdir "$tmp_dir_name"
  dd if=/dev/urandom of="./$tmp_dir_name/$file_name" bs=1M count=50
  popd
  run_failure curl -sS -m 20 --limit-rate 5M -X POST "http://127.0.0.1:$inlet_port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"

  # Consequent attempt fails
  run_failure curl -sS -m 20 -X POST "http://127.0.0.1:$inlet_port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"
}

@test "portals - create inlet with specific identifier" {
  run_success "$OCKAM" node create n
  alt=$("$OCKAM" identity create alt)
  run_success "$OCKAM" tcp-outlet create --to "$PYTHON_SERVER_PORT" --allow "(= subject.identifier \"$alt\")"

  # Create an inlet with the node's identifier, without a secure channel. It shouldn't be allowed to connect
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --from "127.0.0.1:$port" --to /node/n/service/outlet
  run_failure curl -sfI -m 3 "127.0.0.1:$port"

  # Same as before, but through a secure channel. It shouldn't be allowed to connect
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --from "127.0.0.1:$port" --to /node/n/secure/api/service/outlet
  run_failure curl -sfI -m 3 "127.0.0.1:$port"

  # Create an inlet with the alt's identifier. Now it should be allowed to connect
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --from "127.0.0.1:$port" --to /node/n/secure/api/service/outlet --identity alt
  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 2 -m 5 "127.0.0.1:$port"
}
