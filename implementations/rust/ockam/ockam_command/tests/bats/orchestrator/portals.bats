#!/bin/bash

# ===== SETUP

setup() {
  load ../load/base.bash
  load ../load/orchestrator.bash
  load_bats_ext
  setup_home_dir
  skip_if_orchestrator_tests_not_enabled
  copy_enrolled_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS

@test "portals - create an inlet/outlet pair, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue

  run_success "$OCKAM" node create green
  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$relay_name/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - create an inlet using only default arguments, an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name=$(random_str)
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue

  addr=$($OCKAM tcp-inlet create --via $relay_name)
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 $addr
}

@test "portals - create an inlet (with implicit secure channel creation), an outlet, a relay in an orchestrator project and move tcp traffic through it" {
  port="$(random_port)"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "$port" --via "$relay_name"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - inlet/outlet example with credential, not provided" {
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  # Setup nodes from a non-enrolled environment
  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Green isn't enrolled as project member
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  relay_name="$(random_str)"
  run_success "$OCKAM" project-member add "$blue_identifier" --attribute role=member --relay $relay_name
  sleep 2

  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  assert_output --partial "forward_to_$relay_name"

  port="$(random_port)"
  run_success $OCKAM tcp-inlet create --at /node/green --from $port --via $relay_name

  # Green can't establish secure channel with blue, because it didn't exchange credential with it.
  run_failure curl -sfI -m 3 "127.0.0.1:$port"
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

  relay_name="$(random_str)"
  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Green isn't enrolled as project member
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run_success "$OCKAM" project-member add "$blue_identifier" --attribute role=member --relay $relay_name
  sleep 2

  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  assert_output --partial "forward_to_$relay_name"

  run_success "$OCKAM" tcp-inlet create --at /node/green --from "$port" --via "$relay_name"
  # Green can't establish secure channel with blue, because it isn't a member
  run_failure curl -sfI -m 3 "127.0.0.1:$port"
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

  relay_name="$(random_str)"
  run_success "$OCKAM" node create green --identity green
  run_success "$OCKAM" node create blue --identity blue

  # Add identities as members of the project
  export OCKAM_HOME=$ENROLLED_OCKAM_HOME
  run_success "$OCKAM" project-member add "$blue_identifier" --attribute role=member --relay $relay_name
  run_success "$OCKAM" project-member add "$green_identifier" --attribute role=member
  sleep 2

  # Use project from the now enrolled identities
  export OCKAM_HOME=$NON_ENROLLED_OCKAM_HOME
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  assert_output --partial "forward_to_$relay_name"

  run_success bash -c "$OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_$relay_name/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from $port --to -/service/outlet"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - inlet (with implicit secure channel creation) / outlet example with enrollment token" {
  port="$(random_port)"
  ENROLLED_OCKAM_HOME=$OCKAM_HOME

  relay_name="$(random_str)"
  green_token=$($OCKAM project ticket --usage-count 10 --attribute app=app1)
  blue_token=$($OCKAM project ticket --usage-count 10 --attribute app=app1 --relay $relay_name)

  setup_home_dir
  NON_ENROLLED_OCKAM_HOME=$OCKAM_HOME
  #  "$OCKAM" project import --project-file $PROJECT_PATH

  run_success "$OCKAM" identity create green
  run_success "$OCKAM" identity create blue

  run_success "$OCKAM" project enroll $green_token --identity green
  run_success "$OCKAM" node create green --identity green

  run_success "$OCKAM" project enroll $blue_token --identity blue
  run_success "$OCKAM" node create blue --identity blue

  run_success "$OCKAM" tcp-outlet create --at /node/blue \
    --to 127.0.0.1:$PYTHON_SERVER_PORT --allow '(= subject.app "app1")'

  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  assert_output --partial "forward_to_$relay_name"

  run_success "$OCKAM" tcp-inlet create --at /node/green \
    --from "$port" --via "$relay_name" --allow '(= subject.app "app1")'

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$port"
}

@test "portals - local inlet and outlet passing through a relay, removing and re-creating the outlet" {
  inlet_port="$(random_port)"
  node_port="$(random_port)"
  relay_name="$(random_str)"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "$inlet_port" --via "$relay_name"
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  $OCKAM node delete blue --yes
  run_failure curl -sfI -m 3 "127.0.0.1:$inlet_port"

  run_success "$OCKAM" node create blue --tcp-listener-address "127.0.0.1:$node_port"
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" relay create "$relay_name" --to /node/blue
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"
}

@test "portals - create an inlet/outlet pair, copy heavy payload" {
  port="$(random_port)"
  relay_name="$(random_str)"

  run_success "$OCKAM" node create blue
  sleep 1
  run_success "$OCKAM" relay create "${relay_name}" --to /node/blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to "$PYTHON_SERVER_PORT"

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "${port}" \
    --to "/project/default/service/forward_to_${relay_name}/secure/api/service/outlet"

  # generate 10MB of random data
  run_success openssl rand -out "${OCKAM_HOME_BASE}/.tmp/payload" $((1024 * 1024 * 10))

  # write payload to file `payload.copy`
  run_success curl -sf --retry-all-errors --retry-delay 5 --retry 10 -m 60 "127.0.0.1:${port}/.tmp/payload" -o "${OCKAM_HOME}/payload.copy"

  # compare `payload` and `payload.copy`
  run_success cmp "${OCKAM_HOME_BASE}/.tmp/payload" "${OCKAM_HOME}/payload.copy"
}

@test "portals - create an inlet/outlet pair, connection goes down, connection restored" {
  inlet_port="$(random_port)"
  socat_port="$(random_port)"

  project_address=$($OCKAM project show default --output json | jq .access_route -r | sed 's#/dnsaddr/\([^/]*\)/.*#\1#')
  project_port=$($OCKAM project show default --output json | jq .access_route -r | sed 's#.*/tcp/\([^/]*\)/.*#\1#')
  project_id=$($OCKAM project show default --output json | jq .id -r)

  # pass traffic through socat, so we can simulate the connection being interrupted
  socat TCP-LISTEN:${socat_port},reuseaddr TCP:${project_address}:${project_port} &
  socat_pid=$!

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT

  relay_name="$(random_str)"
  run_success "$OCKAM" relay create "${relay_name}" --project-relay --to /node/blue \
    --at "/ip4/127.0.0.1/tcp/${socat_port}/service/$project_id/secure/api"

  run_success "$OCKAM" node create green
  run_success "$OCKAM" tcp-inlet create --at /node/green --from "${inlet_port}" \
    --via "${relay_name}"

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:${inlet_port}"
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert_equal "$status" "Up"

  kill -QUIT $socat_pid
  sleep 1
  run_failure curl -sfI -m 3 "127.0.0.1:${inlet_port}"
  sleep 40
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert [ "$status" != "Up" ]

  # restore connection
  socat TCP-LISTEN:${socat_port},reuseaddr TCP:${project_address}:${project_port} &
  socat_pid=$!
  sleep 2

  run_success curl -sfI --retry-all-errors --retry-delay 2 --retry 10 -m 30 "127.0.0.1:${inlet_port}"
  status=$("$OCKAM" relay show "${relay_name}" --output json | jq .connection_status -r)
  assert_equal "$status" "Up"

  kill -QUIT $socat_pid
}

@test "portals - create a local TLS inlet, https works without skipping verification" {
  skip
  port="$(random_port)"

  ticket=$($OCKAM project ticket --usage-count 10 --tls)
  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create blue
  run_success "$OCKAM" tcp-outlet create --at /node/blue --to 127.0.0.1:$PYTHON_SERVER_PORT
  run_success "$OCKAM" tcp-inlet create --tls --from $port --to /secure/api/service/outlet

  # first wait for the connection without validation
  run_success curl -sfI --insecure --retry-all-errors --retry-delay 5 --retry 10 -m 5 \
    "https://127.0.0.1:${port}"

  # extract certificate subject
  subject=$(openssl s_client -showcerts -connect "127.0.0.1:${port}" </dev/null 2>&1 |
    grep -o 'subject=CN=.*' | sed -E 's/.*CN=[*][.](.*)/\1/')

  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 \
    "https://arbitrary-name.${subject}:${port}"
}
