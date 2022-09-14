
# Install
# =======
# MacOS:
#   brew tap kaos/shell
#   brew install bats-assert
#
# Linux:
#   npm install -g bats bats-support bats-assert
#
# Bats tests can also be run by our Builder Docker image
# ===========
# docker run --rm -it -e HOST_USER_ID=$(id -u) --volume $(pwd):/work ghcr.io/build-trust/ockam-builder:latest bash
# bats implementations/rust/ockam/ockam_command/tests/commands.bats
#
# https://bats-core.readthedocs.io/en/stable/
# https://github.com/ztombol/bats-docs#installation
# https://github.com/ztombol/bats-assert

# This will run local only test:
# bats implementations/rust/ockam/ockam_command/tests/commands.bats
#
# This will run all local only test (including the long ones)
# LONG_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/commands.bats
#
# This expects enroll to have already happened and will run the not very long orchestrator tests
# ORCHESTRATOR_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/commands.bats
#
# This expects enroll to have already happened and will run all orchestrator tests
# ORCHESTRATOR_TESTS=1 LONG_TESTS=1 bats implementations/rust/ockam/ockam_command/tests/commands.bats

# bats_lib=$NVM_DIR/versions/node/v18.8.0/lib/node_modules # linux

# Ockam binary to use
if [[ -z $OCKAM ]]; then
  OCKAM=ockam
fi

if [[ -z $BATS_LIB ]]; then
  BATS_LIB=$(brew --prefix)/lib # macos
fi

# Where node-specific data would be stored, when nodes don't share identities.
# /tmp/blue , /tmp/green , etc.
NODE_PATH=/tmp

setup_file() {
  bats_require_minimum_version 1.5.0

  pushd "$(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir')" &>/dev/null || { echo "pushd failed"; exit 1; }
  python3 -m http.server --bind 127.0.0.1 5000 &
  pid="$!"
  echo "$pid" > "$BATS_FILE_TMPDIR/http_server.pid"
  popd || { echo "popd failed"; exit 1; }
}

teardown_file() {
  pid=$(cat "$BATS_FILE_TMPDIR/http_server.pid")
  kill -9 "$pid"
  wait "$pid" 2>/dev/null || true
}

setup() {
  load "$BATS_LIB/bats-support/load.bash"
  load "$BATS_LIB/bats-assert/load.bash"
  $OCKAM node delete --all || true

  # We want to make sure there is no data (including identities) on these project folders,
  # otherwise we could have a test pass because an identity was _already_ added as
  # a project' member, even when the test itself fails to do so.
  OCKAM_PROJECT_PATH=$NODE_PATH/blue $OCKAM node delete --all -f | true
  OCKAM_PROJECT_PATH=$NODE_PATH/green $OCKAM node delete --all -f | true
  unset OCKAM_PROJECT_PATH
}

teardown() {
  $OCKAM node delete --all || true

  OCKAM_PROJECT_PATH=$NODE_PATH/blue $OCKAM node delete --all -f | true
  OCKAM_PROJECT_PATH=$NODE_PATH/green $OCKAM node delete --all -f | true
  unset OCKAM_PROJECT_PATH
}

@test "create a node without a name" {
  run $OCKAM node create
  assert_success
}

@test "create a node with a name" {
  run $OCKAM node create n1
  assert_success
}

@test "create a node with a name and send it a message" {
  $OCKAM node create n1
  run --separate-stderr $OCKAM message send "hello" --to /node/n1/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create two nodes and send message from one to the other" {
  $OCKAM node create n1
  $OCKAM node create n2

  run --separate-stderr $OCKAM message send "hello" --from n1 --to /node/n2/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create two nodes and send message from one to the other - with /node in --from argument" {
  $OCKAM node create n1
  $OCKAM node create n2

  run --separate-stderr $OCKAM message send "hello" --from /node/n1 --to /node/n2/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a secure channel between two nodes and send message through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api)
  run --separate-stderr $OCKAM message send hello --from /node/n1 --to "$output/service/uppercase"

  assert_success
  assert_output "HELLO"
}

@test "create a secure channel between two nodes and send message through it - in a pipeline" {
  $OCKAM node create n1
  $OCKAM node create n2

  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api | \
    $OCKAM message send hello --from n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a secure channel between three nodes and send message through it - in a pipeline" {
  for i in {1..3}; do $OCKAM node create "n$i"; done

  output=$($OCKAM secure-channel create --from n1 --to /node/n2/node/n3/service/api | \
    $OCKAM message send "hello ockam" --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO OCKAM" ]
}

@test "secure channel with secure channel listener" {
  $OCKAM node create n1
  $OCKAM node create n2

  $OCKAM secure-channel-listener create "listener" --at /node/n2
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/listener | \
    $OCKAM message send hello --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a forwarder and send message through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  $OCKAM forwarder create n1 --at /node/n1 --to /node/n2
  run --separate-stderr $OCKAM message send hello --to /node/n1/service/forward_to_n1/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a forwarder with a dynamic name and send message through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  output=$($OCKAM forwarder create --at /node/n1 --to /node/n2  | \
    $OCKAM message send hello --to /node/n1/-/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create an inlet/outlet pair and move tcp traffic through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  $OCKAM tcp-outlet create --at /node/n1 --from /service/outlet --to 127.0.0.1:5000
  $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:6000 --to /node/n1/service/outlet

  run curl --fail --head 127.0.0.1:6000
  assert_success
}

@test "create an inlet/outlet pair with relay through a forwarder and move tcp traffic through it" {
  $OCKAM node create relay

  $OCKAM node create blue
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  $OCKAM forwarder create blue --at /node/relay --to /node/blue

  $OCKAM node create green
  $OCKAM secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

# the below tests will only succeed if already enrolled with `ockam enroll`

@test "send a message to a project node from command embedded node" {
  skip_if_orchestrator_tests_not_enabled

  run --separate-stderr $OCKAM message send hello --to /project/default/service/echo

  assert_success
  assert_output "hello"
}

@test "send a message to a project node from a spawned background node" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM node create blue
  run --separate-stderr $OCKAM message send hello --from /node/blue --to /project/default/service/echo

  assert_success
  assert_output "hello"
}


@test "list projects" {
  skip_if_orchestrator_tests_not_enabled

  run $OCKAM project list

  assert_success
}

@test "create space, create project, send message, delete project, delete space" {
  skip_if_orchestrator_tests_not_enabled
  skip_if_long_tests_not_enabled

  space_name=$(openssl rand -hex 4)
  project_name=$(openssl rand -hex 4)

  run $OCKAM space create "${space_name}"
  assert_success

  run $OCKAM project create "${space_name}" "${project_name}"
  assert_success

  run --separate-stderr $OCKAM message send hello --to "/project/${project_name}/service/echo"
  assert_success
  assert_output "hello"

  run $OCKAM project delete "${space_name}" "${project_name}"
  assert_success

  run $OCKAM space delete "${space_name}"
  assert_success
}

@test "list spaces" {
  skip_if_orchestrator_tests_not_enabled

  run $OCKAM space list
  assert_success
}


@test "create an inlet/outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM node create blue
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  $OCKAM forwarder create blue --at /project/default --to /node/blue

  $OCKAM node create green
  $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
    | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

@test "inlet/outlet example with credentials, not provided" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project info --name default --output json  > /tmp/project.json

  # Green doesn't enable credentials exchange
  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run $OCKAM node create green --project /tmp/project.json
  assert_success
  green_identifer=$($OCKAM identity show -n green)

  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run $OCKAM node create blue --project /tmp/project.json --enable-credential-checks
  assert_success
  blue_identifer=$(OCKAM_PROJECT_PATH=$NODE_PATH/blue $OCKAM identity show -n blue)

  unset OCKAM_PROJECT_PATH
  run $OCKAM project enroll --member $blue_identifer --to /project/default/service/authenticator
  assert_success
  run $OCKAM project enroll --member $green_identifer --to /project/default/service/authenticator
  assert_success

  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000 --check-credential
  assert_success
  run  $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run bash -c " $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet --check-credential"
  assert_success

  # Green can't establish secure channel with blue, because it doesn't exchange credentials with it.
  run curl --fail --head --max-time 10 127.0.0.1:7000
  assert_failure
}

@test "inlet/outlet example with credentials" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project info --name default --output json  > /tmp/project.json

  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run $OCKAM node create green --project /tmp/project.json --enable-credential-checks
  assert_success
  green_identifer=$($OCKAM identity show -n green)

  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run $OCKAM node create blue --project /tmp/project.json --enable-credential-checks
  assert_success
  blue_identifer=$($OCKAM identity show -n blue)

  unset OCKAM_PROJECT_PATH
  run $OCKAM project enroll --member $blue_identifer --to /project/default/service/authenticator
  assert_success
  run $OCKAM project enroll --member $green_identifer --to /project/default/service/authenticator
  assert_success

  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000 --check-credential
  assert_success
  run  $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run bash -c " $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet --check-credential"
  assert_success

  run curl --fail --head --max-time 10 127.0.0.1:7000
  assert_success
}

@test "project requiring credentials" {
  skip_if_orchestrator_tests_not_enabled
  skip_if_long_tests_not_enabled

  space_name=$(openssl rand -hex 4)
  project_name=$(openssl rand -hex 4)

  run $OCKAM space create "${space_name}"
  assert_success

  run $OCKAM project create "${space_name}" "${project_name}" --enforce-credentials true
  assert_success

  $OCKAM project info --name "${project_name}" --output json  > "/tmp/${project_name}_project.json"

  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run $OCKAM node create green --project "/tmp/${project_name}_project.json" --enable-credential-checks
  assert_success
  green_identifer=$($OCKAM identity show -n green)

  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run $OCKAM node create blue --project "/tmp/${project_name}_project.json"
  assert_success

  # Blue can't create forwarder as it doesn't present credential (it isn't a member neither)
  export OCKAM_PROJECT_PATH=$NODE_PATH/blue
  run  $OCKAM forwarder create blue --at "/project/${project_name}" --to /node/blue
  assert_failure

  # add green as a member
  unset OCKAM_PROJECT_PATH
  run $OCKAM project enroll --member $green_identifer --to "/project/${project_name}/service/authenticator"
  assert_success

  # Now green can access project' services
  export OCKAM_PROJECT_PATH=$NODE_PATH/green
  run  $OCKAM forwarder create green --at "/project/${project_name}" --to /node/green
  assert_success

  unset OCKAM_PROJECT_PATH
  run $OCKAM project delete "${space_name}" "${project_name}"
  assert_success

  run $OCKAM space delete "${space_name}"
  assert_success
}


function skip_if_orchestrator_tests_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi
}

function skip_if_long_tests_not_enabled() {
  if [ -z "${LONG_TESTS}" ]; then
    skip "LONG_TESTS are not enabled"
  fi
}
