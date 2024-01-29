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

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  fwd="$(random_str)"
  run_success "$OCKAM" relay create "$fwd" --to /node/blue

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - create an inlet using only default arguments, an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create --to /node/blue

  addr=$($OCKAM tcp-inlet create)
  run_success curl --fail --head --max-time 10 $addr
}

@test "portals - create an inlet (with implicit secure channel creation), an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  fwd="$(random_str)"
  run_success "$OCKAM" relay create "$fwd" --to /node/blue

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "$fwd"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - inlet/outlet example with credential, not provided" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  # Setup nodes from a non-enrolled environment
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  fwd="$(random_str)"
  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Green isn't enrolled as project member
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run_success "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member --relay $fwd

  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"

  run_success bash -c "$OCKAM secure-channel create --from /node/green --identity green  --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  # Green can't establish secure channel with blue, because it didn't exchange credential with it.
  run_failure curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with credential, not provided" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  fwd="$(random_str)"
  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Green isn't enrolled as project member
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run_success "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member --relay $fwd

  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"

  run_success "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:$port" --to "$fwd"
  # Green can't establish secure channel with blue, because it isn't a member
  run_failure curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - inlet/outlet example with credential" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  # Setup non-enrolled identities
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  fwd="$(random_str)"
  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Add identities as members of the project
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run_success "$OCKAM" project ticket --member "$blue_identifier" --attribute role=member --relay $fwd
  run_success "$OCKAM" project ticket --member "$green_identifier" --attribute role=member

  # Use project from the now enrolled identities
  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"

  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$fwd/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:$port --to -/service/outlet"

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with enrollment token" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  fwd="$(random_str)"
  green_token=$($OCKAM project ticket --attribute app=app1)
  blue_token=$($OCKAM project ticket --attribute app=app1 --relay $fwd)

  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue

  run_success "$OCKAM" project enroll $green_token --identity green
  run_success "$OCKAM" node create green --identity green

  run_success "$OCKAM" project enroll $blue_token --identity blue
  run_success "$OCKAM" node create blue --identity blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue \
    --to 127.0.0.1:$PYTHON_SERVER_PORT --allow '(= subject.app "app1")'

  run_success "$OCKAM" relay create "$fwd" --to /node/blue
  assert_output --partial "forward_to_$fwd"

  run_success "$OCKAM" tcp-inlet create --at /node/green \
    --from "127.0.0.1:$port" --to "$fwd" --allow '(= subject.app "app1")'

  run_success curl --fail --head --max-time 10 "127.0.0.1:$port"
}

@test "portals - local inlet and outlet passing through a relay, removing and re-creating the outlet" {
  port="$(random_port)"
  node_port="$(random_port)"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create --to /node/blue

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green \
    --from "127.0.0.1:$port" --to "/project/default/service/forward_to_default/secure/api/service/outlet"
  run_success curl --fail --head --retry 4 --max-time 10 "127.0.0.1:$port"

  $OCKAM node delete blue --yes
  run_failure curl --fail --head --max-time 5 "127.0.0.1:$port"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create --to /node/blue
  run_success curl --fail --head --retry-connrefused --retry 4 --max-time 10 "127.0.0.1:$port"
}

@test "portals - inlet/outlet with resource type policies" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --relay $relay_name)
  web_ticket=$($OCKAM project ticket --attribute component=web)
  dashboard_ticket=$($OCKAM project ticket --attribute component=dashboard)

  # DB
  setup_home_dir
  DB_OCKAM_HOME=$OCKAM_HOME
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "web")'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp - Has the right attribute, so it should be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name
  run_success curl --head --retry-connrefused --retry 2 --max-time 5 "127.0.0.1:$inlet_port"

  # Dashboard - Doesn't have the right attribute, so it should not be able to connect
  setup_home_dir
  run_success $OCKAM project enroll $dashboard_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name
  run_failure curl --head --retry-connrefused --max-time 5 "127.0.0.1:$inlet_port"
}

@test "portals - inlet/outlet with resource type policies override" {
  # Admin
  relay_name="$(random_str)"
  db_ticket=$($OCKAM project ticket --relay $relay_name)
  web_ticket=$($OCKAM project ticket --attribute component=web)

  # DB
  setup_home_dir
  DB_OCKAM_HOME=$OCKAM_HOME
  run_success $OCKAM project enroll $db_ticket
  run_success $OCKAM relay create $relay_name
  ### Set wrong resource type policy
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "NOT_web")'
  run_success $OCKAM tcp-outlet create --to $PYTHON_SERVER_PORT

  # WebApp
  setup_home_dir
  run_success $OCKAM project enroll $web_ticket
  inlet_port="$(random_port)"
  run_success $OCKAM tcp-inlet create --from $inlet_port --to $relay_name

  # This will fail because the resource type policy is not satisfied
  run_failure curl --head --retry-connrefused --max-time 3 "127.0.0.1:$inlet_port"

  # Update resource type policy and try again. Now the policy is satisfied
  export OCKAM_HOME=$DB_OCKAM_HOME
  run_success $OCKAM policy create --resource-type tcp-outlet --expression '(= subject.component "web")'
  run_success curl --head --retry-connrefused --retry 2 --max-time 5 "127.0.0.1:$inlet_port"

  # Update the policy for the outlet and try again. It will fail because the local policy is not satisfied
  run_success $OCKAM policy create --resource outlet --expression '(= subject.component "NOT_web")'
  run_failure curl --head --retry-connrefused --max-time 3 "127.0.0.1:$inlet_port"
}

@test "portals - create an inlet/outlet pair, copy heavy payload" {
  port="$(random_port)"
  relay_name="$(random_str)"

  run_success "$OCKAM" node create blue
  sleep 1
  run_success "$OCKAM" relay create "${relay_name}" --to /node/blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "127.0.0.1:$PYTHON_SERVER_PORT"

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:${port}" \
    --to "/project/default/service/forward_to_${relay_name}/secure/api/service/outlet"

  # generate 10MB of random data
  run_success openssl rand -out "${OCKAM_HOME_BASE}/payload" $((1024 * 1024 * 10))

  # write payload to file `payload.copy`
  run_success curl --fail --max-time 60 "127.0.0.1:${port}/payload" -o "${OCKAM_HOME}/payload.copy"

  # compare `payload` and `payload.copy`
  run_success cmp "${OCKAM_HOME_BASE}/payload" "${OCKAM_HOME}/payload.copy"
}

@test "portals - create an inlet/outlet pair, connection goes down, connection restored" {
  inlet_port="$(random_port)"
  socat_port="$(random_port)"

  project_address=$(ockam project show default --output json | jq .access_route -r | sed 's#/dnsaddr/\([^/]*\)/.*#\1#')
  project_port=$(ockam project show default --output json | jq .access_route -r | sed 's#.*/tcp/\([^/]*\)/.*#\1#')

  # pass traffic through socat, so we can simulate the connection being interrupted
  socat TCP-LISTEN:${socat_port},reuseaddr TCP:${project_address}:${project_port} &
  socat_pid=$!

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create "${relay_name}" --project-relay --to /node/blue \
    --at "/ip4/127.0.0.1/tcp/${socat_port}/secure/api"

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "127.0.0.1:${inlet_port}" \
    --to "${relay_name}"

  run_success curl --fail --head --max-time 10 "127.0.0.1:${inlet_port}"
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert_equal "$status" "Up"

  kill -INT $socat_pid
  sleep 33

  run_failure curl --fail --head --max-time 1 "127.0.0.1:${inlet_port}"
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert [ "$status" != "Up" ]

  # restore connection
  socat TCP-LISTEN:${socat_port},reuseaddr TCP:${project_address}:${project_port} &
  socat_pid=$!
  sleep 10

  run_success curl --fail --head --max-time 10 "127.0.0.1:${inlet_port}"
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert_equal "$status" "Up"

  kill -INT $socat_pid
}
