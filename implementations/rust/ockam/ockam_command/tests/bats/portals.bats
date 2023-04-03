#!/bin/bash

# ===== SETUP

setup_file() {
  load load/base.bash
}

setup() {
  load load/base.bash
  load load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  load_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "portals - create an inlet/outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  port=7100

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000

  fwd="$(random_str)"
  $OCKAM forwarder create "$fwd" --at /project/default --to /node/blue

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM secure-channel create --from /node/green --to "/project/default/service/forward_to_$fwd/service/api" |
    $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to -/service/outlet

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - create an inlet (with implicit secure channel creation) / outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  port=7101

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000

  fwd="$(random_str)"
  $OCKAM forwarder create "$fwd" --at /project/default --to /node/blue

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - inlet/outlet example with credential, not provided" {
  port=7102
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  # Setup nodes from a non-enrolled environment
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --identity blue
  assert_success

  # Green isn't enrolled as project member
  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project enroll --member "$blue_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/blue
  assert_output --partial "forward_to_$fwd"
  assert_success

  run bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  # Green can't establish secure channel with blue, because it didn't exchange credential with it.
  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_failure
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with credential, not provided" {
  port=7103
  ENROLLED_OCKAM_HOME=$OCKAM_HOME
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH" --identity green
  assert_success

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --identity blue
  assert_success

  # Green isn't enrolled as project member
  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project enroll --member "$blue_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/blue
  assert_output --partial "forward_to_$fwd"
  assert_success

  # Green can't establish secure channel with blue, because it isn't a member
  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  assert_failure
}

@test "portals - inlet/outlet example with credential" {
  port=7104
  ENROLLED_OCKAM_HOME=$OCKAM_HOME
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --identity blue
  assert_success

  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project enroll --member "$blue_identifier" --attribute role=member
  assert_success
  run "$OCKAM" project enroll --member "$green_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/blue
  assert_success
  assert_output --partial "forward_to_$fwd"

  run bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with enrollment token" {
  port=7105
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  green_token=$($OCKAM project enroll --attribute app=app1)
  blue_token=$($OCKAM project enroll --attribute app=app1)

  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success

  run "$OCKAM" project authenticate --project-path "$PROJECT_JSON_PATH" --identity green --token $green_token
  assert_success
  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" policy create --at green --resource tcp-inlet --expression '(= subject.app "app1")'
  assert_success

  run "$OCKAM" project authenticate --project-path "$PROJECT_JSON_PATH" --identity blue --token $blue_token
  assert_success
  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --identity blue
  assert_success
  run "$OCKAM" policy create --at blue --resource tcp-outlet --expression '(= subject.app "app1")'
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" forwarder create "$fwd" --at /project/default --to /node/blue
  assert_output --partial "forward_to_$fwd"
  assert_success

  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}
