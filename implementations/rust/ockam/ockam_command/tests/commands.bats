
# Install
#   brew tap kaos/shell
#   brew install bats-assert
#
# https://bats-core.readthedocs.io/en/stable/
# https://github.com/ztombol/bats-docs#installation
# https://github.com/ztombol/bats-assert

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
  load "$(brew --prefix)/lib/bats-support/load.bash"
  load "$(brew --prefix)/lib/bats-assert/load.bash"
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

@test "create a secure channel between two nodes and send message through it" {
  ockam node create n1
  ockam node create n2

  output=$(ockam secure-channel create --from /node/n1 --to /node/n2/service/api)
  run ockam message send hello --from n1 --to $output/service/uppercase

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
    ockam message send "hello ockam" --from n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO OCKAM" ]
}

@test "secure channel with secure channel listener" {
  ockam node create n1
  ockam node create n2

  ockam secure-channel-listener create "listener" --at /node/n2
  output=$(ockam secure-channel create --from /node/n1 --to /node/n2/service/listener | \
    ockam message send hello --from n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a forwarder and send message through it" {
  ockam node create n1
  ockam node create n2

  ockam forwarder create --from forwarder_to_n2 --for /node/n2 --at /node/n1
  run ockam message send hello --to /node/n1/service/forwarder_to_n2/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create and inlet/outlet pair and move tcp traffic through it" {
  ockam node create n1
  ockam node create n2

  ockam tcp-outlet create --at /node/n1 --from /service/outlet --to 127.0.0.1:5000
  ockam tcp-inlet create --at /node/n2 --from 127.0.0.1:6000 --to /node/n1/service/outlet

  run curl --fail --head 127.0.0.1:6000
  assert_success
}

@test "create and inlet/outlet pair inlet/outlet pair with a relay through a forwarder" {
  ockam node create relay

  ockam node create blue
  ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  ockam forwarder create --at /node/relay --from /service/forwarder_to_blue --for /node/blue

  ockam node create green
  ockam secure-channel create --from /node/green --to /node/relay/service/forwarder_to_blue/service/api \
    | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

# FAILING

# @test "via project" {
#   ockam node create blue
#   ockam tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
#   ockam forwarder create --at /project/default --from /service/forwarder_to_blue --for /node/blue
#
#   ockam node create green
#   ockam secure-channel create --from /node/green --to /project/default/service/forwarder_to_blue/service/api \
#     | ockam tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet
#
#   run curl --fail --head 127.0.0.1:7000
#   assert_success
# }

# @test "create two nodes and send message from one to the other - with /node in --from argument" {
#   ockam node create n1
#   ockam node create n2
#
#   run ockam message send "hello" --from /node/n1 --to /node/n2/service/uppercase
#
#   assert_success
#   assert_output "HELLO"
# }
