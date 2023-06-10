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
  copy_local_orchestrator_data
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "portals - create an inlet/outlet pair, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  fwd="$(random_str)"
  $OCKAM relay create "$fwd" --to /node/blue

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM secure-channel create --from /node/green --to "/project/default/service/forward_to_$fwd/service/api" |
    $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to -/service/outlet

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - create an inlet using only default arguments, an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH"
  assert_success

  $OCKAM tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  $OCKAM relay create --to /node/blue

  addr=$($OCKAM tcp-inlet create)

  run curl --fail --head --max-time 10 $addr
  assert_success
}

@test "portals - create an inlet (with implicit secure channel creation), an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-outlet create --at /node/blue --to 127.0.0.1:5000

  fwd="$(random_str)"
  $OCKAM relay create "$fwd" --to /node/blue

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH"
  assert_success
  $OCKAM tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - inlet/outlet example with credential, not provided" {
  port="$(random_port)"
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

  run "$OCKAM" node create green --project-path "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" node create blue --project-path "$PROJECT_JSON_PATH" --identity blue
  assert_success

  # Green isn't enrolled as project member
  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" relay create "$fwd" --to /node/blue
  assert_success
  assert_output --partial "forward_to_$fwd"

  run bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  # Green can't establish secure channel with blue, because it didn't exchange credential with it.
  run curl --fail --head --max-time 5 "127.0.0.1:$port"
  assert_failure
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with credential, not provided" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run "$OCKAM" node create green --project-path "$PROJECT_JSON_PATH" --identity green
  assert_success

  run "$OCKAM" node create blue --project-path "$PROJECT_JSON_PATH" --identity blue
  assert_success

  # Green isn't enrolled as project member
  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"
  assert_success

  # Green can't establish secure channel with blue, because it isn't a member
  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  assert_failure
}

@test "portals - inlet/outlet example with credential" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run "$OCKAM" node create green --project-path "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" node create blue --project-path "$PROJECT_JSON_PATH" --identity blue
  assert_success

  OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member
  assert_success
  run "$OCKAM" project ticket --member "$green_identifier" --attribute role=member
  assert_success

  OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" relay create "$fwd" --to /node/blue
  assert_success
  assert_output --partial "forward_to_$fwd"

  run bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with enrollment token" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  green_token=$($OCKAM project ticket --attribute app=app1)
  blue_token=$($OCKAM project ticket --attribute app=app1)

  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME

  run "$OCKAM" identity create green
  assert_success
  run "$OCKAM" identity create blue
  assert_success

  run "$OCKAM" project enroll $green_token --identity green
  assert_success
  run "$OCKAM" node create green --project-path "$PROJECT_JSON_PATH" --identity green
  assert_success
  run "$OCKAM" policy create --at green --resource tcp-inlet --expression '(= subject.app "app1")'
  assert_success

  run "$OCKAM" project enroll $blue_token --identity blue
  assert_success
  run "$OCKAM" node create blue --project-path "$PROJECT_JSON_PATH" --identity blue
  assert_success
  run "$OCKAM" policy create --at blue --resource tcp-outlet --expression '(= subject.app "app1")'
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  fwd="$(random_str)"
  run "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"
  assert_success

  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "/project/default/service/forward_to_$fwd/secure/api/service/outlet"
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success
}


@test "portals - local inlet and outlet, removing and re-creating the outlet" {
  port="$(random_port)"
  node_port="$(random_port)"

  setup_home_dir

  run "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  run "$OCKAM" node create green
  assert_success

  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to /node/blue/secure/api/service/outlet
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success

  run "$OCKAM" node delete blue
  assert_success

  run curl --fail --head --max-time 2 "127.0.0.1:$port"
  assert_failure

  run "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  run curl --head --retry-connrefused --retry 20 --retry-max-time 20 --max-time 1 "127.0.0.1:$port"
  assert_success
}


@test "portals - local inlet and outlet passing though a relay, removing and re-creating the outlet" {
  port="$(random_port)"
  node_port="$(random_port)"

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --tcp-listener-address "127.0.0.1:$node_port"
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  run "$OCKAM" relay create --to /node/blue
  assert_success

  run "$OCKAM" node create green --project "$PROJECT_JSON_PATH"
  assert_success

  run "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to /project/default/service/forward_to_default/secure/api/service/outlet
  assert_success

  run curl --fail --head --max-time 10 "127.0.0.1:$port"
  assert_success

  $OCKAM node delete blue

  run curl --fail --head --max-time 2 "127.0.0.1:$port"
  assert_failure

  run "$OCKAM" node create blue --project "$PROJECT_JSON_PATH" --tcp-listener-address "127.0.0.1:$node_port"
  assert_success

  run "$OCKAM" relay create --to /node/blue
  assert_success

  run "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:5000
  assert_success

  run curl --head --retry-connrefused --retry 50 --max-time 1 "127.0.0.1:$port"
  assert_success
}
