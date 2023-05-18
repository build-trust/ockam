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

# ===== NODE

@test "node - create with random name" {
  run "$OCKAM" node create
  assert_success
}

@test "node - create with name" {
  n="$(random_str)"
  run "$OCKAM" node create "$n"
  assert_success

  run "$OCKAM" node show "$n"
  assert_success
  assert_output --partial "/dnsaddr/localhost/tcp/"
  assert_output --partial "/service/api"
  assert_output --partial "/service/uppercase"
}

@test "node - start services" {
  run "$OCKAM" node create n1
  assert_success

  # Check we can start service, but only once with the same name
  run "$OCKAM" service start identity my_identity --node n1
  assert_success
  run "$OCKAM" service start identity my_identity --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run "$OCKAM" service start authenticated my_authenticated --node n1
  assert_success
  run "$OCKAM" service start authenticated my_authenticated --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run "$OCKAM" service start verifier --addr my_verifier --node n1
  assert_success
  run "$OCKAM" service start verifier --addr my_verifier --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run "$OCKAM" service start credentials --addr my_credentials --node n1 --identity 0134dabe4f886af3bd5d2b3ab50891a6dfe90c99099668ce8cb680888cac7d67db000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e1acf2670f5bfc34c466910949618c68a53183976e8e57d5fc07b6a3d02d22a3030101407e6332d0deeccf8d12de9972e31b54200f1597db2a195d08b15b251d6293c180611c66acc26913a16d5ea5536227c8baefb4fa95bd709212fdc1ca4fc3370e02
  assert_success
  run "$OCKAM" service start credentials --addr my_credentials --node n1 --identity 0134dabe4f886af3bd5d2b3ab50891a6dfe90c99099668ce8cb680888cac7d67db000547c93239ba3d818ec26c9cdadd2a35cbdf1fa3b6d1a731e06164b1079fb7b8084f434b414d5f524b03012000000020e1acf2670f5bfc34c466910949618c68a53183976e8e57d5fc07b6a3d02d22a3030101407e6332d0deeccf8d12de9972e31b54200f1597db2a195d08b15b251d6293c180611c66acc26913a16d5ea5536227c8baefb4fa95bd709212fdc1ca4fc3370e02
  assert_failure
}

@test "node - is restarted with default services" {
  n="$(random_str)"
  # Create node, check that it has one of the default services running
  run "$OCKAM" node create "$n"
  assert_success
  assert_output --partial "/service/identity_service"

  # Stop node, restart it, and check that the service is up again
  $OCKAM node stop "$n"
  run "$OCKAM" node start "$n"
  assert_success
  assert_output --partial "/service/identity_service"
}

# ===== VAULT

@test "vault - create and check show/list output" {
  v1=$(random_str)
  run "$OCKAM" vault create "${v1}"
  assert_success

  run "$OCKAM" vault show "${v1}"
  assert_success
  assert_output --partial "Name: ${v1}"
  assert_output --partial "Type: OCKAM"

  v2=$(random_str)
  run "$OCKAM" vault create "${v2}" --aws-kms
  assert_success

  run "$OCKAM" vault show "${v2}"
  assert_success
  assert_output --partial "Name: ${v2}"
  assert_output --partial "Type: AWS KMS"

  run "$OCKAM" vault list
  assert_success
  assert_output --partial "Name: ${v1}"
  assert_output --partial "Type: OCKAM"
  assert_output --partial "Name: ${v2}"
  assert_output --partial "Type: AWS KMS"
}

@test "vault - CRUD" {
  # Create with random name
  run "$OCKAM" vault create
  assert_success

  # Create with specific name
  v=$(random_str)

  run "$OCKAM" vault create "${v}"
  assert_success
  run "$OCKAM" vault delete "${v}"
  assert_success
  run "$OCKAM" vault show "${v}"
  assert_failure

  # Delete vault and leave identities untouched
  v=$(random_str)
  i=$(random_str)

  run "$OCKAM" vault create "${v}"
  assert_success
  run "$OCKAM" identity create "${i}" --vault "${v}"
  assert_success
  run "$OCKAM" vault delete "${v}"
  assert_success
  run "$OCKAM" vault show "${v}"
  assert_failure
  run "$OCKAM" identity show "${i}"
  assert_success
}

# ===== IDENTITY

@test "identity - create and check show output" {
  i=$(random_str)
  run "$OCKAM" identity create "${i}"
  assert_success

  run "$OCKAM" identity show "${i}"
  assert_success
  assert_output --regexp '^P'

  run "$OCKAM" identity show "${i}" --full
  assert_success
  assert_output --partial "Change History"
  assert_output --partial "identifier"
  assert_output --partial "signatures"
}

@test "identity - CRUD" {
  # Create with random name
  run "$OCKAM" identity create
  assert_success

  # Create a named identity and delete it
  i=$(random_str)
  run "$OCKAM" identity create "${i}"
  assert_success

  run "$OCKAM" identity delete "${i}"
  assert_success

  # Fail to delete identity when it's in use by a node
  i=$(random_str)
  n=$(random_str)

  run "$OCKAM" identity create "${i}"
  assert_success
  run "$OCKAM" node create "${n}" --identity "${i}"
  assert_success
  run "$OCKAM" identity delete "${i}"
  assert_failure

  # Delete identity after deleting the node
  run "$OCKAM" node delete "${n}"
  assert_success
  run "$OCKAM" identity delete "${i}"
  assert_success
}

# ===== TCP

@test "tcp connection - CRUD" {
  run "$OCKAM" node create n1
  assert_success

  # Create tcp-connection and check output
  run "$OCKAM" tcp-connection create --from n1 --to 127.0.0.1:5000 --output json
  assert_success
  assert_output --regexp '[{"route":"/dnsaddr/localhost/tcp/[[:digit:]]+/worker/[[:graph:]]+"}]'

  # Check that the connection is listed
  run "$OCKAM" tcp-connection list --node n1
  assert_success
  assert_output --partial "$id"

  id=$($OCKAM tcp-connection list --node n1 | grep -o "[0-9a-f]\{32\}" | head -1)

  # Show the connection details
  run "$OCKAM" tcp-connection show --node n1 "$id"
  assert_success
  assert_output --partial "$id"

  # Delete the connection
  run "$OCKAM" tcp-connection delete --node n1 "$id"
  assert_success

  # Check that it's no longer listed
  run "$OCKAM" tcp-connection list --node n1
  assert_success
  refute_output --partial "$id"
}

@test "tcp listener - CRUD" {
  run "$OCKAM" node create n1
  assert_success

  # Create tcp-listener and check output
  run "$OCKAM" tcp-listener create --at n1 127.0.0.1:7000
  assert_success
  assert_output --regexp '/dnsaddr/localhost/tcp/[[:digit:]]+'

  # Check that the listener is listed
  run "$OCKAM" tcp-listener list --node n1
  assert_success
  assert_output --partial "127.0.0.1:7000"

  addr=$($OCKAM tcp-listener list --node n1 | tail -3 | head -1 | grep -o "[0-9a-f]\{32\}" | head -1)

  # Show the listener details
  run "$OCKAM" tcp-listener show --node n1 "$addr"
  assert_success
  assert_output --partial "$addr"

#  # Delete the listener
  run "$OCKAM" tcp-listener delete --node n1 "$addr"
  assert_success

  # Check that it's no longer listed
  run "$OCKAM" tcp-listener list --node n1
  assert_success
  refute_output --partial "$addr"
}

@test "tcp - create a tcp connection and then delete it" {
  run "$OCKAM" node create n1
  run "$OCKAM" tcp-connection create --from n1 --to 127.0.0.1:5000 --output json
  assert_success

}

# ===== MESSAGE

@test "message - send messages between local nodes" {
  # Send from a temporary node to a background node
  run "$OCKAM" node create n1
  assert_success
  msg=$(random_str)
  run "$OCKAM" message send "$msg" --timeout 5 --to /node/n1/service/uppercase
  assert_success
  assert_output "$(to_uppercase "$msg")"

  # Send between two background nodes
  run "$OCKAM" node create n2
  assert_success
  msg=$(random_str)
  run "$OCKAM" message send "$msg" --timeout 5 --from n1 --to /node/n2/service/uppercase
  assert_success
  assert_output "$(to_uppercase "$msg")"

  # Same, but using the `/node/` prefix in the `--from` argument
  msg=$(random_str)
  run "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to /node/n2/service/uppercase
  assert_success
  assert_output "$(to_uppercase "$msg")"
}

@test "message - secure-channels with authorized identifiers" {
  run "$OCKAM" vault create v1
  assert_success
  run "$OCKAM" identity create i1 --vault v1
  assert_success
  idt1=$($OCKAM identity show i1)

  run "$OCKAM" vault create v2
  assert_success
  run "$OCKAM" identity create i2 --vault v2
  assert_success
  idt2=$($OCKAM identity show i2)

  run "$OCKAM" node create n1 --vault v1 --identity i1
  assert_success
  run "$OCKAM" node create n2 --vault v1 --identity i1
  assert_success

  msg=$(random_str)
  run "$OCKAM" secure-channel-listener create l --at n2 --vault v2 --identity i2 --authorized "$idt1"
  run bash -c " $OCKAM secure-channel create --from n1 --to /node/n2/service/l --authorized $idt2 \
              | $OCKAM message send $msg --from /node/n1 --to -/service/echo"
  assert_success
  assert_output "$msg"
}

# ===== SECURE CHANNEL

@test "secure channel - create secure channel and send message through it" {
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  # In two separate commands
  msg=$(random_str)
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api)
  run "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to "$output/service/uppercase"
  assert_success
  assert_output "$(to_uppercase "$msg")"

  # Piping the output of the first command into the second
  msg=$(random_str)
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api |
    $OCKAM message send "$msg" --from /node/n1 --to -/service/uppercase)
  assert [ "$output" == "$(to_uppercase "$msg")" ]

  # Using an explicit secure channel listener
  $OCKAM secure-channel-listener create n2scl --at /node/n2
  msg=$(random_str)
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/n2scl |
    $OCKAM message send "$msg" --from /node/n1 --to -/service/uppercase)
  assert [ "$output" == "$(to_uppercase "$msg")" ]
}

@test "secure channel - send message directly using secure multiaddr" {
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  msg=$(random_str)
  run "$OCKAM" message send "$msg" --timeout 5 --from /node/n1 --to "/node/n2/secure/api/service/uppercase"
  assert_success
  assert_output "$(to_uppercase "$msg")"
}

# ===== RELAY

@test "relay - create relay with default parameters" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  port=7100

  run "$OCKAM" node create blue
  assert_success
  $OCKAM tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  fwd="$(random_str)"
  run "$OCKAM" relay create $fwd
  assert_success

  run "$OCKAM" node create green
  assert_success
  $OCKAM secure-channel create --from /node/green --to "/project/default/service/forward_to_$fwd/service/api" |
    $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to -/service/outlet

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "relay - create relay and send message through it" {
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  # In two separate commands
  $OCKAM relay create n2 --at /node/n1 --to /node/n2
  msg=$(random_str)
  run "$OCKAM" message send --timeout 5 "$msg" --to /node/n1/service/forward_to_n2/service/uppercase
  assert_success
  assert_output "$(to_uppercase "$msg")"

  # Piping the output of the first command into the second
  msg=$(random_str)
  output=$($OCKAM relay create --at /node/n2 --to /node/n1 |
    $OCKAM message send "$msg" --to /node/n2/-/service/uppercase)
  assert [ "$output" == "$(to_uppercase "$msg")" ]
}

@test "relay - create two relays and list them on a node" {
  run --separate-stderr "$OCKAM" node create n1
  assert_success
  run --separate-stderr "$OCKAM" node create n2
  assert_success

  run $OCKAM relay create blue --at /node/n1 --to /node/n2
  assert_success
  run $OCKAM relay create red --at /node/n1 --to /node/n2
  assert_success

  run $OCKAM relay list --at /node/n2
  assert_output --regexp "Relay Route:.* => 0#forward_to_blue"
  assert_output --partial "Remote Address: /service/forward_to_blue"
  assert_output --regexp "Worker Address: /service/.*"
  assert_output --regexp "Relay Route:.* => 0#forward_to_red"
  assert_output --partial "Remote Address: /service/forward_to_red"
  assert_output --regexp "Worker Address: /service/.*"
  assert_success

  # Test listing node with no relays
  run $OCKAM relay list --at /node/n1
  assert_output --partial "No relays found on node n1"
  assert_failure
}

@test "relay - show a relay on a node" {
  run --separate-stderr "$OCKAM" node create n1
  assert_success
  run --separate-stderr "$OCKAM" node create n2
  assert_success

  run $OCKAM relay create blue --at /node/n1 --to /node/n2
  assert_success

  run $OCKAM relay show forward_to_blue --at /node/n2
  assert_output --regexp "Relay Route:.* => 0#forward_to_blue"
  assert_output --partial "Remote Address: /service/forward_to_blue"
  assert_output --regexp "Worker Address: /service/.*"
  assert_success

  # Test showing non-existing with no relay
  run $OCKAM relay show forwarder_blank --at /node/n2
  assert_output --partial "NotFound"
  assert_failure
}

# ===== PORTALS (INLET/OUTLET)

@test "portals - tcp inlet CRUD" {
  outlet_port=6100
  inlet_port=6101

  # Create nodes for inlet/outlet pair
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  # Create inlet/outlet pair
  run $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$outlet_port" --alias "test-outlet"
  assert_output --partial "/service/outlet"
  assert_success

  run $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$inlet_port --to /node/n1/service/outlet --alias "test-inlet"
  assert_success

  run $OCKAM tcp-inlet create --at /node/n2 --from 6102 --to /node/n1/service/outlet
  assert_success

  # Check that inlet is available for deletion and delete it
  run $OCKAM tcp-inlet show test-inlet --node /node/n2
  assert_output --partial "Alias: test-inlet"
  assert_output --partial "TCP Address: 127.0.0.1:$inlet_port"
  assert_output --regexp "To Outlet Address: /service/.*/service/outlet"
  assert_success

  run $OCKAM tcp-inlet delete "test-inlet" --node /node/n2
  assert_success

  # Test deletion of a previously deleted TCP inlet
  run $OCKAM tcp-inlet delete "test-inlet" --node /node/n2
  assert_output --partial "NotFound"
}

@test "portals - tcp outlet CRUD" {
  port=6103
  run "$OCKAM" node create n1
  assert_success

  only_port=6104
  run "$OCKAM" node create n2
  assert_success

  run $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"
  assert_success

  run $OCKAM tcp-outlet create --at /node/n2 --to $only_port
  assert_success

  run $OCKAM tcp-outlet show test-outlet --node /node/n1
  assert_output --partial "Alias: test-outlet"
  assert_output --partial "From Outlet: /service/outlet"
  assert_output --regexp "To TCP: 127.0.0.1:$port"
  assert_success

  run $OCKAM tcp-outlet delete "test-outlet"
  assert_success

  # Test deletion of a previously deleted TCP outlet
  run $OCKAM tcp-outlet delete "test-outlet"
  assert_output --partial "NotFound"
}

@test "portals - list inlets on a node" {
  port=6104
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  run $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias tcp-inlet-2
  run $OCKAM tcp-inlet list --node /node/n2

  assert_output --partial "Alias: tcp-inlet-2"
  assert_output --partial "TCP Address: 127.0.0.1:$port"
  assert_output --regexp "To Outlet Address: /service/.*/service/outlet"
  assert_success
}

@test "portals - list outlets on a node" {
  port=6105
  run "$OCKAM" node create n1

  run $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"
  assert_success

  run $OCKAM tcp-outlet list --node /node/n1
  assert_output --partial "Alias: test-outlet"
  assert_output --partial "From Outlet: /service/outlet"
  assert_output --regexp "To TCP: 127.0.0.1:$port"
  assert_success
}

@test "portals - show a tcp inlet" {
  port=6106
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  run $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:$port --to /node/n1/service/outlet --alias "test-inlet"
  assert_success

  run $OCKAM tcp-inlet show "test-inlet" --node /node/n2
  assert_success

  # Test if non-existing TCP inlet returns NotFound
  run $OCKAM tcp-inlet show "non-existing-inlet"
  assert_output --partial "NotFound"
}

@test "portals - show a tcp outlet" {
  port=6107
  run "$OCKAM" node create n1
  assert_success

  run $OCKAM tcp-outlet create --at /node/n1 --to "127.0.0.1:$port" --alias "test-outlet"
  assert_output --partial "/service/outlet"
  assert_success

  run $OCKAM tcp-outlet show "test-outlet"
  assert_success

  # Test if non-existing TCP outlet returns NotFound
  run $OCKAM tcp-outlet show "non-existing-outlet"
  assert_output --partial "NotFound"
}

@test "portals - create an inlet/outlet pair and move tcp traffic through it" {
  port=6000
  run "$OCKAM" node create n1
  assert_success
  run "$OCKAM" node create n2
  assert_success

  $OCKAM tcp-outlet create --at /node/n1 --to 127.0.0.1:5000
  $OCKAM tcp-inlet create --at /node/n2 --from "127.0.0.1:$port" --to /node/n1/service/outlet

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - create an inlet/outlet pair with relay through a relay and move tcp traffic through it" {
  port=6001
  run "$OCKAM" node create relay
  assert_success
  run "$OCKAM" node create blue
  assert_success

  $OCKAM tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  $OCKAM relay create blue --at /node/relay --to /node/blue

  run "$OCKAM" node create green
  assert_success
  $OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api |
    $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to -/service/outlet

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success

  run "$OCKAM" secure-channel list --at green
  assert_success
  assert_output --partial "/service"
}
