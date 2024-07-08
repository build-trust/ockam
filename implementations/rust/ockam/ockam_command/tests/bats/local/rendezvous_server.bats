#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  kill $RENDEZVOUS_SERVER_PID
  teardown_home_dir
}

# ===== TESTS

@test "rendezvous server - rendezvous server responds to a healthcheck" {
  port="$(random_port)"
  "$OCKAM" rendezvous-server start --healthcheck="127.0.0.1:$port" &
  RENDEZVOUS_SERVER_PID=$!

  sleep 1

  run_success curl -m 5 "127.0.0.1:$port"
  assert_output --partial "Alive"
}

@test "rendezvous server - local TCP portal over UDP puncture" {
  port="$(random_port)"
  inlet_port="$(random_port)"
  "$OCKAM" rendezvous-server start --udp="127.0.0.1:$port" --healthcheck="127.0.0.1:0" &
  RENDEZVOUS_SERVER_PID=$!

  sleep 1

  export OCKAM_RENDEZVOUS_SERVER="127.0.0.1:$port"

  run_success "$OCKAM" node create bob --enable-udp
  run_success "$OCKAM" tcp-outlet create --at bob --to "$PYTHON_SERVER_PORT"

  run_success "$OCKAM" node create alice --enable-udp
  run_success "$OCKAM" tcp-inlet create --at alice --enable-udp-puncture --disable-tcp-fallback --from "$inlet_port" --to /node/bob/secure/api/service/outlet

  run_success curl -sfI --retry-connrefused --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}
