#!/bin/bash

# https://docs.ockam.io/use-cases

# ===== SETUP

setup_file() {
  load load/base.bash
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
  run "$OCKAM" node create relay
  assert_success

  # Service
  run "$OCKAM" node create server_sidecar
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/server_sidecar --to 127.0.0.1:5000
  assert_success
  run "$OCKAM" relay create server_sidecar --at /node/relay --to /node/server_sidecar
  assert_output --partial "forward_to_server_sidecar"
  assert_success

  # Client
  run "$OCKAM" node create client_sidecar
  assert_success
  run bash -c "$OCKAM secure-channel create --from /node/client_sidecar --to /node/relay/service/forward_to_server_sidecar/service/api \
              | $OCKAM tcp-inlet create --at /node/client_sidecar --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

# https://docs.ockam.io/
@test "use-case - end-to-end encryption, orchestrator" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

  port=9001

  # Service
  run "$OCKAM" node create s
  run "$OCKAM" tcp-outlet create --at /node/s --to 127.0.0.1:5000

  fwd=$(random_str)
  run "$OCKAM" relay create "$fwd" --to /node/s

  # Client
  run "$OCKAM" node create c
  run bash -c "$OCKAM secure-channel create --from /node/c --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/c --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

# https://docs.ockam.io/use-cases/apply-fine-grained-permissions-with-attribute-based-access-control-abac
@test "use-case - abac" {
  skip_if_orchestrator_tests_not_enabled
  copy_local_orchestrator_data

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
  fwd=$(random_str)
  $OCKAM identity create control_identity
  $OCKAM project authenticate $cp1_token --project-path "$PROJECT_JSON_PATH" --identity control_identity
  $OCKAM node create control_plane1 --project-path "$PROJECT_JSON_PATH" --identity control_identity
  $OCKAM policy create --at control_plane1 --resource tcp-outlet --expression '(= subject.component "edge")'
  $OCKAM tcp-outlet create --at /node/control_plane1 --to 127.0.0.1:5000
  run "$OCKAM" relay create "$fwd" --to /node/control_plane1
  assert_success

  # Edge plane
  setup_home_dir
  $OCKAM identity create edge_identity
  $OCKAM project authenticate $ep1_token --project-path "$PROJECT_JSON_PATH" --identity edge_identity
  $OCKAM node create edge_plane1 --project-path "$PROJECT_JSON_PATH" --identity edge_identity
  $OCKAM policy create --at edge_plane1 --resource tcp-inlet --expression '(= subject.component "control")'
  $OCKAM tcp-inlet create --at /node/edge_plane1 --from "127.0.0.1:$port_1" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  run curl --fail --head --max-time 5 "127.0.0.1:$port_1"
  assert_success

  ## The following is denied
  $OCKAM identity create x_identity
  $OCKAM project authenticate $x_token --project-path "$PROJECT_JSON_PATH" --identity x_identity
  $OCKAM node create x --project-path "$PROJECT_JSON_PATH" --identity x_identity
  $OCKAM policy create --at x --resource tcp-inlet --expression '(= subject.component "control")'
  $OCKAM tcp-inlet create --at /node/x --from "127.0.0.1:$port_2" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  run curl --fail --head --max-time 5 "127.0.0.1:$port_2"
  assert_failure 28 # timeout error
}
