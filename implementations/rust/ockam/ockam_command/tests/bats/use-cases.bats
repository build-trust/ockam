#!/bin/bash

# https://docs.ockam.io/use-cases

# ===== SETUP

setup_file() {
  load load/base.bash
  load load/orchestrator.bash
  load_orchestrator_data
}

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

# https://docs.ockam.io/guides/use-cases/add-end-to-end-encryption-to-any-client-and-server-application-with-no-code-change
@test "use-case - end-to-end encryption, local" {
  port=9000
  run --separate-stderr "$OCKAM" node create relay
  assert_success

  # Service
  run --separate-stderr "$OCKAM" node create server_sidecar
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/server_sidecar --from /service/outlet --to 127.0.0.1:5000
  assert_success
  run "$OCKAM" forwarder create server_sidecar --at /node/relay --to /node/server_sidecar
  assert_output --partial "forward_to_server_sidecar"
  assert_success

  # Client
  run --separate-stderr "$OCKAM" node create client_sidecar
  assert_success
  run bash -c "$OCKAM secure-channel create --from /node/client_sidecar --to /node/relay/service/hop/service/forward_to_server_sidecar/service/api \
              | $OCKAM tcp-inlet create --at /node/client_sidecar --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

# https://docs.ockam.io/
@test "use-case - end-to-end encryption, orchestrator" {
  skip_if_orchestrator_tests_not_enabled
  copy_orchestrator_data

  port=9001

  # Service
  run "$OCKAM" node create s --project "$PROJECT_JSON_PATH"
  run "$OCKAM" tcp-outlet create --at /node/s --from /service/outlet --to 127.0.0.1:5000

  fwd=$(random_str)
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/s

  # Client
  run "$OCKAM" node create c --project "$PROJECT_JSON_PATH"
  run bash -c "$OCKAM secure-channel create --from /node/c --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/c --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

# https://docs.ockam.io/use-cases/apply-fine-grained-permissions-with-attribute-based-access-control-abac
@test "use-case - abac" {
  skip "TODO: Update after latest ABAC changes"
  skip_if_orchestrator_tests_not_enabled
  copy_orchestrator_data

  port_1=9002
  port_2=9003

  # Administrator
  ADMIN_OCKAM_HOME=$OCKAM_HOME
  cp1_token=$($OCKAM project enroll --attribute component=control)
  ep1_token=$($OCKAM project enroll --attribute component=edge)
  x_token=$($OCKAM project enroll --attribute component=x)

  # Control plane
  setup_home_dir
  CONTROL_OCKAM_HOME=$OCKAM_HOME
  run "$OCKAM" node create control_plane1 --project "$PROJECT_JSON_PATH" --enrollment-token "$cp1_token"
  run "$OCKAM" policy set --at control_plane1 --resource tcp-outlet --expression '(= subject.component "edge")'
  run "$OCKAM" tcp-outlet create --at /node/control_plane1 --from /service/outlet --to 127.0.0.1:5000

  fwd=$(random_str)
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/control_plane1

  # Edge plane
  setup_home_dir
  run "$OCKAM" node create edge_plane1 --project "$PROJECT_JSON_PATH" --enrollment-token "$ep1_token"
  run "$OCKAM" policy set --at edge_plane1 --resource tcp-inlet --expression '(= subject.component "control")'
  run "$OCKAM" tcp-inlet create --at /node/edge_plane1 --from "127.0.0.1:$port_1" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  run curl --fail --head --max-time 5 "127.0.0.1:$port_1"
  assert_success

  ## The following is denied
  run "$OCKAM" node create x --project "$PROJECT_JSON_PATH" --enrollment-token "$x_token"
  run "$OCKAM" policy set --at x --resource tcp-inlet --expression '(= subject.component "control")'
  run "$OCKAM" tcp-inlet create --at /node/x --from "127.0.0.1:$port_2" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  run curl --fail --head --max-time 5 "127.0.0.1:$port_2"
  assert_failure 28 # timeout error

  teardown_home_dir
  OCKAM_HOME=$CONTROL_OCKAM_HOME
  teardown_home_dir
  OCKAM_HOME=$ADMIN_OCKAM_HOME
}
