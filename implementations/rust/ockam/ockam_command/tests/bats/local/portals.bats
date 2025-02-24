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

@test "portals - create inlet at random port" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  addr=$("$OCKAM" tcp-inlet create inlet --at /node/n2 --to /node/n1/service/outlet --jq '.bind_address')

  addr=${addr//\"/}
  host_port=(${addr//:/ })
  [[ "${host_port[0]}" == "127.0.0.1" ]] || fail "Host should be 127.0.0.1, got: ${host_port[0]}"
  [[ "${host_port[1]}" != "0" ]] || fail "Port should be other than 0, got ${host_port[1]}"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "$addr"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - create an inlet/outlet, download file" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT"
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet

  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./.tmp/$file_name" bs=1M count=50 && popd
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 10 -m 20 -o /dev/null "http://127.0.0.1:$port/.tmp/$file_name"
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
  run_success curl -sS --retry-all-errors --retry-delay 5 --retry 10 -m 20 -X POST "http://127.0.0.1:$port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it, where the outlet points to an HTTPs endpoint" {
  if [[ "$OCKAM_PRIVILEGED" = "1" ]]; then
    skip "OCKAM_PRIVILEGE not supported"
  fi

  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # rust-lang.org rejects http requests to its port 443
  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to rust-lang.org:443 --tls
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --http-header 'Host: rust-lang.org' --at /node/n2 --from "$port" --to /node/n1/service/outlet

  run_success curl --fail --head --max-time 10 "127.0.0.1:${port}"
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

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"

  run_success "$OCKAM" secure-channel list --at green
  assert_output --partial "/service"
}

@test "portals no handshake - create an inlet/outlet pair and move tcp traffic through it" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT" --skip-handshake
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet --skip-handshake

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals no handshake - create an inlet/outlet, download file" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT" --skip-handshake
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet --skip-handshake

  file_name="$(random_str)".bin
  pushd "$OCKAM_HOME_BASE" && dd if=/dev/urandom of="./.tmp/$file_name" bs=1M count=50 && popd
  run_success curl -sSf --retry-all-errors --retry-delay 5 --retry 10 -m 20 -o /dev/null "http://127.0.0.1:$port/.tmp/$file_name"
}

@test "portals no handshake - create an inlet/outlet, upload file" {
  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to "$PYTHON_SERVER_PORT" --skip-handshake
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --at /node/n2 --from "$port" --to /node/n1/service/outlet --skip-handshake

  file_name="$(random_str)".bin
  tmp_dir_name="$(random_str)"
  pushd "$OCKAM_HOME_BASE/.tmp"
  mkdir "$tmp_dir_name"
  dd if=/dev/urandom of="./$tmp_dir_name/$file_name" bs=1M count=50
  popd
  run_success curl -sS --retry-all-errors --retry-delay 5 --retry 10 -m 20 -X POST "http://127.0.0.1:$port/upload" -F "files=@$OCKAM_HOME_BASE/.tmp/$tmp_dir_name/$file_name"
}

@test "portals no handshake - create an inlet/outlet pair and move tcp traffic through it, where the outlet points to an HTTPs endpoint" {
  if [[ "$OCKAM_PRIVILEGED" = "1" ]]; then
    skip "OCKAM_PRIVILEGE not supported"
  fi

  run_success "$OCKAM" node create n1
  run_success "$OCKAM" node create n2

  # rust-lang.org rejects http requests to its port 443
  run_success "$OCKAM" tcp-outlet create --at /node/n1 --to rust-lang.org:443 --skip-handshake --tls
  port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --skip-handshake --http-header 'Host: rust-lang.org' --at /node/n2 --from "$port" --to /node/n1/service/outlet

  run_success curl --fail --head --max-time 10 "127.0.0.1:${port}"
}

@test "portals no handshake - create an inlet/outlet pair with relay through a relay and move tcp traffic through it" {
  run_success "$OCKAM" node create relay
  run_success "$OCKAM" node create blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT" --skip-handshake
  run_success "$OCKAM" relay create blue --at /node/relay --to /node/blue

  run_success "$OCKAM" node create green
  port="$(random_port)"
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet --skip-handshake"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"

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
  run_success "$OCKAM" tcp-inlet create i --at n --from "$port" --to "/node/n/service/outlet"
  port="$(random_port)"
  run_failure "$OCKAM" tcp-inlet create i --at n --from "$port" --to "/node/n/service/outlet"
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
  run_success "$OCKAM" tcp-inlet create --ping-timeout 500ms --at /node/green --from "$inlet_port" --to /node/blue/secure/api/service/outlet
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  run_success "$OCKAM" node delete blue --yes
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"

  sleep 1
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}

@test "portals - local inlet and outlet in reverse order" {
  run_success "$OCKAM" node create n1
  node_port="$(random_port)"
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create --ping-timeout 500ms --at /node/n1 --from "${inlet_port}" --to "/ip4/127.0.0.1/tcp/${node_port}/service/outlet"

  run_success "$OCKAM" node create n2 --tcp-listener-address "127.0.0.1:${node_port}"
  run_success "$OCKAM" tcp-outlet create --at /node/n2 --to "$PYTHON_SERVER_PORT"

  sleep 1
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:${inlet_port}"
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
  run_failure curl -sSf -m 20 --limit-rate 5M -o /dev/null "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl -sSf -m 20 -o /dev/null "http://127.0.0.1:$inlet_port/.tmp/$file_name"
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
  run_failure curl -sSf -m 20 --limit-rate 5M -o /dev/null "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null

  # Consequent attempt fails
  run_failure curl -sSf -m 20 -o /dev/null "http://127.0.0.1:$inlet_port/.tmp/$file_name" >/dev/null
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
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 2 -m 5 "127.0.0.1:$port"
}

@test "portals - http set header" {
  if [[ "$OCKAM_PRIVILEGED" = "1" ]]; then
    skip "OCKAM_PRIVILEGE not supported"
  fi

  inlet_port="$(random_port)"
  server_port="$(random_port)"

  # to validate the header has been set, we start an authenticated HTTP server
  # and we inject the credential into the request
  run_success "$OCKAM" tcp-outlet create --to "127.0.0.1:${server_port}"
  run_success "$OCKAM" tcp-inlet create --from "127.0.0.1:${inlet_port}" --to /service/outlet

  uploadserver --basic-auth username:password --bind 127.0.0.1 ${server_port} &>"$HOME/.bats-tests/authenticated_python_server.log" &
  echo $! >"${OCKAM_HOME}/authenticated_python_server.pid"

  wait_for_port ${server_port}
  wait_for_port ${inlet_port}

  run_failure curl -sf -m 3 "http://127.0.0.1:${inlet_port}"

  authentication="$(echo -n "username:password" | base64)"
  run_success "$OCKAM" tcp-inlet delete --all --yes

  wait_for_closed_port ${inlet_port}
  run_success "$OCKAM" tcp-inlet create --http-header "Authorization: Basic ${authentication}" --from "127.0.0.1:${inlet_port}" --to /service/outlet

  wait_for_port ${inlet_port}
  run_success curl -sf -m 5 "http://127.0.0.1:${inlet_port}"
}

@test "portals - highly available portal keep working after relay is deleted" {
  # blue is the outlet node
  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"

  # create relays nodes
  run_success "$OCKAM" node create relay1
  run_success "$OCKAM" node create relay2

  run_success "$OCKAM" relay create --at /node/relay1 --to /node/blue blue1
  run_success "$OCKAM" relay create --at /node/relay2 --to /node/blue blue2

  # green is the inlet node
  run_success "$OCKAM" node create green
  inlet_port="$(random_port)"
  run_success "$OCKAM" tcp-inlet create \
    --from "$inlet_port" \
    --at /node/green \
    --ping-timeout 500ms \
    --to /node/relay1/secure/api/service/forward_to_blue1/secure/api/service/outlet \
    --to /node/relay2/secure/api/service/forward_to_blue2/secure/api/service/outlet

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  # delete relay1
  run_success "$OCKAM" node delete relay1 --yes

  # sleep to make sure the timeout is triggered
  sleep 1

  # check that the connection is still working by querying multiple times
  for i in {1..10}; do
    # no retray, as we want to check that every connection is successful
    run_success curl -sfI -m 1 "127.0.0.1:$inlet_port"
  done
}
