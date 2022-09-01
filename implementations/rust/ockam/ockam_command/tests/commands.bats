
# Install
# =======
# MacOS:
#   brew tap kaos/shell
#   brew install bats-assert
#
# Linux:
#   npm install -g bats bats-support bats-assert
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
bats_lib=$(brew --prefix)/lib # macos

setup_file() {
  pushd $(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir') &>/dev/null
  python3 -m http.server --bind 127.0.0.1 5000 &
  pid="$!"
  echo "$pid" > "$BATS_FILE_TMPDIR/http_server.pid"
  popd
}

teardown_file() {
  pid=$(cat "$BATS_FILE_TMPDIR/http_server.pid")
  kill -9 "$pid"
  wait "$pid" 2>/dev/null || true
}

setup() {
  load "$bats_lib/bats-support/load.bash"
  load "$bats_lib/bats-assert/load.bash"
  ockam node delete --all || true
}

teardown() {
  ockam node delete --all || true
}

@test "create a node without a name" {
  ockam node create
  assert_success
}

@test "create a node with a name" {
  ockam node create n1
  assert_success
}

@test "create a node with a name and send it a message" {
  ockam node create n1
  run ockam message send "hello" --to /node/n1/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create two nodes and send message from one to the other" {
  ockam node create n1
  ockam node create n2

  run ockam message send "hello" --from n1 --to /node/n2/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create two nodes and send message from one to the other - with /node in --from argument" {
  ockam node create n1
  ockam node create n2

  run ockam message send "hello" --from /node/n1 --to /node/n2/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a secure channel between two nodes and send message through it" {
  ockam node create n1
  ockam node create n2

  output=$(ockam secure-channel create --from /node/n1 --to /node/n2/service/api)
  run ockam message send hello --from /node/n1 --to $output/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a secure channel between two nodes and send message through it - in a pipeline" {
  ockam node create n1
  ockam node create n2

  output=$(ockam secure-channel create --from /node/n1 --to /node/n2/service/api | \
    ockam message send hello --from n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a secure channel between three nodes and send message through it - in a pipeline" {
  for i in {1..3}; do ockam node create "n$i"; done

  output=$(ockam secure-channel create --from n1 --to /node/n2/node/n3/service/api | \
    ockam message send "hello ockam" --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO OCKAM" ]
}

@test "secure channel with secure channel listener" {
  ockam node create n1
  ockam node create n2

  ockam secure-channel-listener create "listener" --at /node/n2
  output=$(ockam secure-channel create --from /node/n1 --to /node/n2/service/listener | \
    ockam message send hello --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a forwarder and send message through it" {
  ockam node create n1
  ockam node create n2

  ockam forwarder create n1 --at /node/n1 --to /node/n2
  run ockam message send hello --to /node/n1/service/forward_to_n1/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a forwarder with a dynamic name and send message through it" {
  ockam node create n1
  ockam node create n2

  output=$(ockam forwarder create --at /node/n1 --to /node/n2  | \
    ockam message send hello --to /node/n1/-/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create an inlet/outlet pair and move tcp traffic through it" {
  ockam node create n1
  ockam node create n2

  ockam tcp-outlet create --at /node/n1 --from /service/outlet --to 127.0.0.1:5000
  ockam tcp-inlet create --at /node/n2 --from 127.0.0.1:6000 --to /node/n1/service/outlet

  run curl --fail --head 127.0.0.1:6000
  assert_success
}

@test "create an inlet/outlet pair with relay through a forwarder and move tcp traffic through it" {
  ockam node create relay

  ockam node create blue
  ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  ockam forwarder create blue --at /node/relay --to /node/blue

  ockam node create green
  ockam secure-channel create --from /node/green --to /node/relay/service/forward_to_blue/service/api \
    | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

# the below tests will succeed if already enrolled with
# ockam enroll
#

@test "send a message to a project node from command embedded node" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  run ockam message send hello --to /project/default/service/echo

  assert_success
  assert_output "hello"
}

@test "send a message to a project node from a spawned background node" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  ockam node create blue
  run ockam message send hello --from /node/blue --to /project/default/service/echo

  assert_success
  assert_output "hello"
}


@test "list projects" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  run ockam project list

  assert_success
}

@test "create space, create project, send message, delete project, delete space" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  if [ -z "${LONG_TESTS}" ]; then
    skip "LONG_TESTS are not enabled"
  fi

  space_name=$(openssl rand -hex 4)
  project_name=$(openssl rand -hex 4)

  run ockam space create ${space_name}
  assert_success

  run ockam project create ${space_name} ${project_name}
  assert_success

  run ockam message send hello --to /project/${project_name}/service/echo
  assert_success
  assert_output "hello"

  run ockam project delete ${space_name} ${project_name}
  assert_success

  run ockam space delete ${space_name}
  assert_success
}

@test "list spaces" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  run ockam space list

  assert_success
}


@test "create an inlet/outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  if [ -z "${ORCHESTRATOR_TESTS}" ]; then
    skip "ORCHESTRATOR_TESTS are not enabled"
  fi

  ockam node create blue
  ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  ockam forwarder create blue --at /project/default --to /node/blue

  ockam node create green
  ockam secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
    | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}
