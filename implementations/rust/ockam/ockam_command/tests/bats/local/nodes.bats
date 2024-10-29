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

@test "node - create with random name" {
  run_success "$OCKAM" node create
}

@test "node - create with name" {
  run_success "$OCKAM" node create n

  run_success "$OCKAM" node show n
  assert_output --partial "\"name\": \"n\""
  assert_output --partial "/dnsaddr/localhost/tcp/"
  assert_output --partial "\"addr\": \"uppercase\""
}

@test "node - start services" {
  run_success "$OCKAM" node create n1

  # Check we can start service, but only once with the same name
  run_success "$OCKAM" service start hop --addr my_hop --at n1
  run_failure "$OCKAM" service start hop --addr my_hop --at n1
}

@test "node - is restarted with default services" {
  # Create node, check that it has one of the default services running
  run_success "$OCKAM" node create n

  # Stop node, restart it, and check that the service is up again
  $OCKAM node stop n
  run_success "$OCKAM" node start n
  assert_output --partial "\"addr\": \"echo\""
}

@test "node - fail to create two background nodes with the same name" {
  run_success "$OCKAM" node create n
  run_failure "$OCKAM" node create n
}

@test "node - can recreate a background node after it was gracefully stopped" {
  run_success "$OCKAM" node create n
  run_success "$OCKAM" node stop n
  # Recreate node
  run_success "$OCKAM" node create n
}

@test "node - can recreate a background node after it was killed" {
  # This test emulates the situation where a node is killed by the OS
  # on a restart or a shutdown. The node should be able to restart without errors.
  run_success "$OCKAM" node create n

  force_kill_node n

  # Recreate node
  run_success "$OCKAM" node create n
}

@test "node - fail to create two foreground nodes with the same name" {
  run_success "$OCKAM" node create n -f &
  sleep 2
  run_success "$OCKAM" node show n
  run_failure "$OCKAM" node create n -f
}

@test "node - can recreate a foreground node after it was killed" {
  run_success "$OCKAM" node create n -f &
  sleep 2
  run_success "$OCKAM" node show n

  force_kill_node n

  # Recreate node
  run_success "$OCKAM" node create n -f &
  sleep 2
  run_success "$OCKAM" node show n
}

@test "node - can recreate a foreground node after it was gracefully stopped" {
  run_success "$OCKAM" node create n -f &
  sleep 2
  run_success "$OCKAM" node show n

  run_success "$OCKAM" node stop n

  # Recreate node
  run_success "$OCKAM" node create n -f &
  sleep 2
  run_success "$OCKAM" node show n
}

@test "node - background node logs to file" {
  run_success "$OCKAM" node create n
  run_success ls -l "$OCKAM_HOME/nodes/n"
  assert_output --partial "stdout"
}

@test "node - foreground node logs to stdout only" {
  run_success "$OCKAM" node create n -vv -f &
  sleep 2
  # It should even create the node directory
  run_failure ls -l "$OCKAM_HOME/nodes/n"
}

@test "node - create a node with an inline configuration" {
  run_success "$OCKAM" node create --configuration "{name: n, tcp-outlets: {db-outlet: {to: 5432, at: n}}}"
  run_success $OCKAM node show n --output json
  assert_output --partial "\"name\": \"n\""
  assert_output --partial "127.0.0.1:5432"

  run_success "$OCKAM" node create "{name: o, tcp-outlets: {db-outlet: {to: 5433, at: o}}}"
  run_success $OCKAM node show o --output json
  assert_output --partial "\"name\": \"o\""
  assert_output --partial "127.0.0.1:5433"

  run_success "$OCKAM" node create "name: p"
  run_success $OCKAM node show p --output json
  assert_output --partial "\"name\": \"p\""

  run_success "$OCKAM" node create "name: q" --foreground &
  sleep 3
  run_success $OCKAM node show q --output json
  assert_output --partial "\"name\": \"q\""
}

@test "node - node in foreground with configuration is deleted if something fails" {
  # The config file has invalid port to trigger an error after the node is created.
  # The command should return an error and the node should be deleted.
  run_failure "$OCKAM" node create --configuration "{name: n, tcp-outlets: {db-outlet: {to: \"localhost:65536\"}}}"
  run_success $OCKAM node show n --output json
  assert_output --partial "[]"

  run_failure "$OCKAM" node create "{name: n, tcp-outlets: {db-outlet: {to: \"localhost:65536\"}}}"
  run_success $OCKAM node show n --output json
  assert_output --partial "[]"
}

@test "node - create two nodes with the same inline configuration" {
  run_success "$OCKAM" node create --configuration "{tcp-outlets: {to: 8080}}"
  run_success "$OCKAM" node create --configuration "{tcp-outlets: {to: 8080}}"

  # each node must have its own outlet
  node_names="$($OCKAM node list --output json | jq -r 'map(.node_name) | join(" ")')"
  for node_name in $node_names; do
    run_success $OCKAM node show $node_name --output json
    assert_output --partial 8080
  done
}

@test "node - return error if passed variable has no value" {
  run_failure "$OCKAM" node create --configuration "{name: n}" --variable MY_VAR=
  assert_output --partial "Empty value for variable 'MY_VAR'"
}

@test "node - the HTTP server is disabled by default" {
  run_success $OCKAM node create
  run_success $OCKAM node show --output json
  cmd_output="$output"
  http_addr="$(echo $cmd_output | jq -r .http_server_address)"
  assert_equal "$http_addr" "null"
}

@test "node - check the contents returned from the HTTP server endpoints" {
  run_success $OCKAM node create --http-server
  run_success $OCKAM node show --output json
  cmd_output="$output"
  http_addr="$(echo $cmd_output | jq -r .http_server_address)"
  run_success curl -fsI -m 2 $http_addr
  run_failure curl -fsI -m 2 $http_addr/show
  run_success curl -fs -m 2 $http_addr/show
}

@test "node - the HTTP server is enabled with a boolean flag and a random port is assigned to it" {
  run_success $OCKAM node create --http-server
  run_success $OCKAM node show --output json
  http_addr="$(echo $output | jq -r .http_server_address)"
  run_success curl -fsI -m 2 $http_addr
}

@test "node - the HTTP server is enabled with a specific port" {
  port=$(random_port)
  run_success $OCKAM node create --http-server-port $port
  run_success curl -fsI -m 2 127.0.0.1:$port
}

@test "node - fail to create node with invalid name" {
  run_failure "$OCKAM" node create n!
  run_failure "$OCKAM" node create n.1
  run_failure "$OCKAM" node create ./config.yaml
  run_failure "$OCKAM" node create node.yaml

  # The previous will work if the file exists
  cat <<EOF >"$OCKAM_HOME/node.yaml"
name: n1
EOF
  run_success "$OCKAM" node create "$OCKAM_HOME/node.yaml"
}
