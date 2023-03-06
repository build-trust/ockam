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

  pushd "$(mktemp -d 2>/dev/null || mktemp -d -t 'tmpdir')" &>/dev/null || {
    echo "pushd failed"
    exit 1
  }
  python3 -m http.server --bind 127.0.0.1 5000 &
  pid="$!"
  echo "$pid" >"$BATS_FILE_TMPDIR/http_server.pid"
  popd || {
    echo "popd failed"
    exit 1
  }
}

teardown_file() {
  pid=$(cat "$BATS_FILE_TMPDIR/http_server.pid")
  kill -9 "$pid"
  wait "$pid" 2>/dev/null || true
}

setup() {
  load "$BATS_LIB/bats-support/load.bash"
  load "$BATS_LIB/bats-assert/load.bash"
  unset OCKAM_HOME
  $OCKAM node delete --all || true
  OCKAM_HOME=/tmp/ockam $OCKAM node delete --all || true
  rm -rf /tmp/ockam
}

teardown() {
  unset OCKAM_HOME
  $OCKAM node delete --all || true
  OCKAM_HOME=/tmp/ockam $OCKAM node delete --all || true
  rm -rf /tmp/ockam
}

@test "create a node without a name" {
  run $OCKAM node create
  assert_success
}

@test "node is restarted with default services" {
  # Create node, check that it has one of the default services running
  run $OCKAM node create n1
  assert_success
  assert_output --partial "/service/vault_service"

  # Stop node, restart it, and check that the service is up again
  $OCKAM node stop n1
  run $OCKAM node start n1
  assert_success
  assert_output --partial "/service/vault_service"
}

@test "create a vault and do show on it" {
  vault_name1=$(openssl rand -hex 4)
  run $OCKAM vault create "${vault_name1}"
  assert_success

  run $OCKAM vault show "${vault_name1}"
  assert_success
  assert_output --partial "Name: ${vault_name1}"
  assert_output --partial "Type: OCKAM"

  vault_name2=$(openssl rand -hex 4)
  run $OCKAM vault create "${vault_name2}" --aws-kms
  assert_success

  run $OCKAM vault show "${vault_name2}"
  assert_success
  assert_output --partial "Name: ${vault_name2}"
  assert_output --partial "Type: AWS KMS"

  run $OCKAM vault list
  assert_success
  assert_output --partial "Name: ${vault_name1}"
  assert_output --partial "Type: OCKAM"
  assert_output --partial "Name: ${vault_name2}"
  assert_output --partial "Type: AWS KMS"
}

@test "create a identity and do show on it" {
  idt_name=$(openssl rand -hex 4)
  run $OCKAM identity create "${idt_name}"
  assert_success

  run $OCKAM identity show "${idt_name}"
  assert_success
  assert_output --regexp '^P'
}

@test "create a identity and do show change history on it" {
  idt_name=$(openssl rand -hex 4)
  run $OCKAM identity create "${idt_name}"
  assert_success

  run $OCKAM identity show "${idt_name}" --full
  assert_success
  assert_output --partial "Change History"
  assert_output --partial "identifier"
  assert_output --partial "signatures"
}

@test "create a node with a name and do show on it" {
  run $OCKAM node create n1
  assert_success

  run $OCKAM node show n1
  assert_success
  assert_output --partial "/dnsaddr/localhost/tcp/"
  assert_output --partial "/service/api"
  assert_output --partial "/service/uppercase"
}

@test "create a node with a set of pre-trusted identities" {
  # TODO:  authenticated command doesn't know anything about how to resolve 'node' multiaddr
  run $OCKAM identity create t1
  test_identity=$($OCKAM identity show t1)
  run $OCKAM node create n1 --tcp-listener-address 127.0.0.1:6001 --trusted-identities "{\"$test_identity\": {\"sample_attr\": \"sample_val\"}}"
  assert_success
  run $OCKAM authenticated get --id $test_identity /dnsaddr/127.0.0.1/tcp/6001/service/authenticated
  assert_success
  assert_output --partial "sample_val"
  run $OCKAM authenticated list /dnsaddr/127.0.0.1/tcp/6001/service/authenticated
  assert_success
  assert_output --partial "sample_val"
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

@test "vault CRUD" {
  # Create with random name
  run $OCKAM vault create
  assert_success

  # Create with specific name
  vault_name=$(openssl rand -hex 4)

  run $OCKAM vault create "${vault_name}"
  assert_success
  run $OCKAM vault delete "${vault_name}"
  assert_success
  run $OCKAM vault show "${vault_name}"
  assert_failure

  # Delete vault and leave identities untouched
  vault_name=$(openssl rand -hex 4)
  idt_name=$(openssl rand -hex 4)

  run $OCKAM vault create "${vault_name}"
  assert_success
  run $OCKAM identity create "${idt_name}" --vault "${vault_name}"
  assert_success
  run $OCKAM vault delete "${vault_name}"
  assert_success
  run $OCKAM vault show "${vault_name}"
  assert_failure
  run $OCKAM identity show "${idt_name}"
  assert_success
}

@test "identity CRUD" {
  # Create with random name
  run $OCKAM identity create
  assert_success
  assert_output --partial "Identity created"

  # Create a named identity and delete it
  idt_name=$(openssl rand -hex 4)
  run $OCKAM identity create "${idt_name}"
  assert_success
  assert_output --partial "Identity created"
  run $OCKAM identity delete "${idt_name}"
  assert_success
  assert_output --partial "Identity '${idt_name}' deleted"

  # Fail to delete identity when it's in use by a node
  idt_name=$(openssl rand -hex 4)
  node_name=$(openssl rand -hex 4)

  run $OCKAM identity create "${idt_name}"
  assert_success
  run $OCKAM node create "${node_name}" --identity "${idt_name}"
  assert_success
  run $OCKAM identity delete "${idt_name}"
  assert_failure

  # Delete identity after deleting the node
  run $OCKAM node delete "${node_name}"
  assert_success
  run $OCKAM identity delete "${idt_name}"
  assert_success
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

  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api |
    $OCKAM message send hello --from n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a secure channel between three nodes and send message through it - in a pipeline" {
  for i in {1..3}; do $OCKAM node create "n$i"; done

  output=$($OCKAM secure-channel create --from n1 --to /node/n2/serice/hop/node/n3/service/api |
    $OCKAM message send "hello ockam" --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO OCKAM" ]
}

@test "secure channel with secure channel listener" {
  $OCKAM node create n1
  $OCKAM node create n2

  $OCKAM secure-channel-listener create "listener" --at /node/n2
  output=$($OCKAM secure-channel create --from /node/n1 --to /node/n2/service/listener |
    $OCKAM message send hello --from /node/n1 --to -/service/uppercase)

  assert [ "$output" == "HELLO" ]
}

@test "create a forwarder and send message through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  $OCKAM forwarder create n1 --at /node/n1 --to /node/n2
  run --separate-stderr $OCKAM message send hello --to /node/n1/service/hop/service/forward_to_n1/service/uppercase

  assert_success
  assert_output "HELLO"
}

@test "create a forwarder with a dynamic name and send message through it" {
  $OCKAM node create n1
  $OCKAM node create n2

  output=$($OCKAM forwarder create --at /node/n1 --to /node/n2 |
    $OCKAM message send hello --to /node/n1/service/hop/-/service/uppercase)

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

@test "list inlets on a node" {
  $OCKAM node create n1
  $OCKAM node create n2
  $OCKAM tcp-inlet create --at /node/n2 --from 127.0.0.1:6000 --to /node/n1/service/outlet --alias tcp-inlet-2
  run $OCKAM tcp-inlet list --node /node/n2

  assert_output --partial "Alias: tcp-inlet-2"
  assert_output --partial "TCP Address: 127.0.0.1:6000"
  assert_output --regexp "To Outlet Address: /service/.*/service/outlet"
}

@test "create an inlet/outlet pair with relay through a forwarder and move tcp traffic through it" {
  $OCKAM node create relay

  $OCKAM node create blue
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  $OCKAM forwarder create blue --at /node/relay --to /node/blue

  $OCKAM node create green
  $OCKAM secure-channel create --from /node/green --to /node/relay/service/hop/service/forward_to_blue/service/api |
    $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

@test "create a node and start services" {
  $OCKAM node create n1

  # Check we can start service, but only once with the same name
  run $OCKAM service start vault my_vault --node n1
  assert_success
  run $OCKAM service start vault my_vault --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run $OCKAM service start identity my_identity --node n1
  assert_success
  run $OCKAM service start identity my_identity --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run $OCKAM service start authenticated my_authenticated --node n1
  assert_success
  run $OCKAM service start authenticated my_authenticated --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run $OCKAM service start verifier --addr my_verifier --node n1
  assert_success
  run $OCKAM service start verifier --addr my_verifier --node n1
  assert_failure

  # Check we can start service, but only once with the same name
  run $OCKAM service start credentials --addr my_credentials --node n1
  assert_success
  run $OCKAM service start credentials --addr my_credentials --node n1
  assert_failure

  # TODO: add test for authenticator
}

@test "create a tcp connection" {
  run $OCKAM node create n1
  run $OCKAM tcp-connection create --from n1 --to 127.0.0.1:5000 --output json
  assert_success
  assert_output --regexp '[{"route":"/dnsaddr/localhost/tcp/[[:digit:]]+/worker/[[:graph:]]+"}]'

  run $OCKAM tcp-connection list --node n1
  assert_success
  assert_output --partial "127.0.0.1:5000"
}

@test "create a tcp connection and then delete it " {
  run $OCKAM node create n1
  run $OCKAM tcp-connection create --from n1 --to 127.0.0.1:5000 --output json
  assert_success
  id=$($OCKAM tcp-connection list --node n1 | grep -o "[0-9a-f]\{32\}" | head -1)
  run $OCKAM tcp-connection delete --node n1 $id
  assert_success
  assert_output "Tcp connection \`$id\` successfully deleted"
  run $OCKAM tcp-connection list --node n1
  assert_success
  refute_output --partial "127.0.0.1:5000"
}

@test "standalone authority, enrollers, members" {
  run $OCKAM identity create authority
  run $OCKAM identity create enroller
  # m1 will be pre-authenticated on authority.  m2 will be added directly, m3 will be added through enrollment token
  run $OCKAM identity create m1
  run $OCKAM identity create m2
  run $OCKAM identity create m3
  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)
  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  # Create a launch configuration json file,  to be used to start the authority node
  echo '{"startup_services" : {"authenticator" : {"project" : "1"}, "secure_channel_listener": {}}}' >/tmp/auth_launch_config.json

  # Start the authority node.  We pass a set of pre trusted-identities containing m1' identity identifier

  run $OCKAM node create --tcp-listener-address=127.0.0.1:4200 --identity authority --launch-config /tmp/auth_launch_config.json --trusted-identities "{\"$m1_identifier\": {\"sample_attr\" : \"sample_val\", \"project_id\" : \"1\"}, \"$enroller_identifier\" : {\"project_id\" : \"1\", \"ockam-role\" : \"enroller\"}}" authority
  assert_success

  echo "{\"id\": \"1\",
  \"name\" : \"default\",
  \"identity\" : \"P6c20e814b56579306f55c64e8747e6c1b4a53d9a3f4ca83c252cc2fbfc72fa94\",
  \"access_route\" : \"/dnsaddr/127.0.0.1/tcp/4000/service/api\",
  \"authority_access_route\" : \"/dnsaddr/127.0.0.1/tcp/4200/service/api\",
  \"authority_identity\" : \"$authority_identity_full\"}" >/tmp/project.json

  # m1 is a member (its on the set of pre-trusted identifiers) so it can get it's own credential
  run $OCKAM project authenticate --project-path /tmp/project.json --identity m1
  assert_success
  assert_output --partial "sample_val"

  run $OCKAM project enroll --identity enroller --project-path /tmp/project.json --member $m2_identifier --attribute sample_attr=m2_member
  assert_success

  run $OCKAM project authenticate --project-path /tmp/project.json --identity m2
  assert_success
  assert_output --partial "m2_member"

  token=$($OCKAM project enroll --identity enroller --project-path /tmp/project.json --attribute sample_attr=m3_member)
  run $OCKAM project authenticate --project-path /tmp/project.json --identity m3 --token $token
  assert_success
  assert_output --partial "m3_member"
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

@test "send a message to a project node from command embedded node, passing identity" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information --output json >/tmp/project.json

  run $OCKAM identity create m1
  run $OCKAM identity create m2
  m1_identifier=$($OCKAM identity show m1)

  $OCKAM project enroll --member $m1_identifier --attribute role=member

  # m1' identity was added by enroller
  run $OCKAM project authenticate --identity m1 --project-path /tmp/project.json

  # m1 is a member,  must be able to contact the project' service
  run --separate-stderr $OCKAM message send --identity m1 --project-path /tmp/project.json --to /project/default/service/echo hello
  assert_success
  assert_output "hello"

  # m2 is not a member,  must not be able to contact the project' service
  run --separate-stderr $OCKAM message send --identity m2 --project-path /tmp/project.json --to /project/default/service/echo hello
  assert_failure
}

@test "send a message to a project node from command embedded node, enrolled member on different install" {
  skip # FIXME  how to send a message to a project m1 is enrolled to?  (with m1 being on a different install
  #       than the admin?.  If we pass project' address directly (instead of /project/ thing), would
  #       it present credential? would read authority info from project.json?
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information --output json >/tmp/project.json

  export OCKAM_HOME=/tmp/ockam
  $OCKAM identity create m1
  $OCKAM identity create m2
  m1_identifier=$($OCKAM identity show m1)

  unset OCKAM_HOME
  $OCKAM project enroll --member $m1_identifier --attribute role=member

  export OCKAM_HOME=/tmp/ockam
  # m1' identity was added by enroller
  run $OCKAM project authenticate --identity m1 --project-path /tmp/project.json

  # m1 is a member,  must be able to contact the project' service
  run --separate-stderr $OCKAM message send --identity m1 --project-path /tmp/project.json --to /project/default/service/echo hello
  assert_success
  assert_output "hello"

  # m2 is not a member,  must not be able to contact the project' service
  run --separate-stderr $OCKAM message send --identity m2 --project-path /tmp/project.json --to /project/default/service/echo hello
  assert_failure
}

@test "list projects" {
  skip_if_orchestrator_tests_not_enabled

  run $OCKAM project list

  assert_success
}

@test "list spaces" {
  skip_if_orchestrator_tests_not_enabled

  run $OCKAM space list
  assert_success
}

@test "project enrollment" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information default --output json >/tmp/project.json

  export OCKAM_HOME=/tmp/ockam
  run $OCKAM identity create green
  run $OCKAM identity create blue

  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  # They haven't been added by enroller yet
  run $OCKAM project authenticate --identity green --project-path /tmp/project.json
  assert_failure

  unset OCKAM_HOME
  $OCKAM project enroll --member $green_identifier --attribute role=member
  blue_token=$($OCKAM project enroll --attribute role=member)

  export OCKAM_HOME=/tmp/ockam

  # Green' identity was added by enroller
  run $OCKAM project authenticate --identity green --project-path /tmp/project.json
  assert_success
  assert_output --partial $green_identifier

  # For blue, we use an enrollment token generated by enroller
  run $OCKAM project authenticate --identity blue --token $blue_token --project-path /tmp/project.json
  assert_success
  assert_output --partial $blue_identifier
}

@test "create an inlet/outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM node create blue
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  $OCKAM forwarder create blue --at /project/default --to /node/blue

  $OCKAM node create green
  $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api |
    $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

@test "create an inlet (with implicit secure channel creation) / outlet pair with relay through a forwarder in an orchestrator project and move tcp traffic through it" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM node create blue
  $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  $OCKAM forwarder create blue --at /project/default --to /node/blue

  $OCKAM node create green
  $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to /project/default/service/forward_to_blue/secure/api/service/outlet

  run curl --fail --head 127.0.0.1:7000
  assert_success
}

@test "inlet/outlet example with credential, not provided" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information default --output json >/tmp/project.json

  export OCKAM_HOME=/tmp/ockam

  run $OCKAM identity create green
  run $OCKAM identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run $OCKAM node create green --project /tmp/project.json --identity green
  assert_success

  run $OCKAM node create blue --project /tmp/project.json --identity blue
  assert_success

  # Green isn't enrolled as project member
  unset OCKAM_HOME
  run $OCKAM project enroll --member $blue_identifier --attribute role=member
  assert_success

  export OCKAM_HOME=/tmp/ockam
  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success

  run $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  run bash -c " $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet"
  assert_success

  unset OCKAM_HOME

  # Green can't establish secure channel with blue, because it doesn't exchange credential with it.
  run curl --fail --head --max-time 10 127.0.0.1:7000
  assert_failure
}

@test "inlet (with implicit secure channel creation) / outlet example with credential, not provided" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information default --output json >/tmp/project.json

  export OCKAM_HOME=/tmp/ockam

  run $OCKAM identity create green
  run $OCKAM identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run $OCKAM node create green --project /tmp/project.json --identity green
  assert_success

  run $OCKAM node create blue --project /tmp/project.json --identity blue
  assert_success

  # Green isn't enrolled as project member
  unset OCKAM_HOME
  run $OCKAM project enroll --member $blue_identifier --attribute role=member
  assert_success

  export OCKAM_HOME=/tmp/ockam
  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success
  run $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  # Green can't establish secure channel with blue, because it isn't a member
  run $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to /project/default/service/forward_to_blue/secure/api/service/outlet
  assert_failure
  unset OCKAM_HOME
}

@test "inlet/outlet example with credential" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information default --output json >/tmp/project.json

  OCKAM_HOME=/tmp/ockam

  run $OCKAM identity create green
  run $OCKAM identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run $OCKAM node create green --project /tmp/project.json --identity green
  assert_success

  run $OCKAM node create blue --project /tmp/project.json --identity blue
  assert_success

  unset OCKAM_HOME
  run $OCKAM project enroll --member $blue_identifier --attribute role=member
  assert_success
  run $OCKAM project enroll --member $green_identifier --attribute role=member
  assert_success

  OCKAM_HOME=/tmp/ockam
  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success
  run $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  run bash -c " $OCKAM secure-channel create --from /node/green --to /project/default/service/forward_to_blue/service/api \
              | $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to -/service/outlet"
  assert_success
  unset OCKAM_HOME

  run curl --fail --head --max-time 10 127.0.0.1:7000
  assert_success
}

@test "inlet (with implicit secure channel creation) / outlet example with enrollment token" {
  skip_if_orchestrator_tests_not_enabled

  $OCKAM project information default --output json >/tmp/project.json

  green_token=$($OCKAM project enroll --attribute app=app1)
  blue_token=$($OCKAM project enroll --attribute app=app1)

  export OCKAM_HOME=/tmp/ockam

  run $OCKAM identity create green
  run $OCKAM identity create blue

  run $OCKAM project authenticate --project-path /tmp/project.json --identity green --token $green_token
  assert_success
  run $OCKAM node create green --project /tmp/project.json --identity green
  assert_success
  run $OCKAM policy set --at green --resource tcp-inlet --expression '(= subject.app "app1")'
  assert_success

  run $OCKAM project authenticate --project-path /tmp/project.json --identity blue --token $blue_token
  assert_success
  run $OCKAM node create blue --project /tmp/project.json --identity blue
  assert_success
  run $OCKAM policy set --at blue --resource tcp-outlet --expression '(= subject.app "app1")'
  assert_success

  run $OCKAM tcp-outlet create --at /node/blue --from /service/outlet --to 127.0.0.1:5000
  assert_success
  run $OCKAM forwarder create blue --at /project/default --to /node/blue
  assert_output --partial "forward_to_blue"
  assert_success

  run $OCKAM tcp-inlet create --at /node/green --from 127.0.0.1:7000 --to /project/default/service/forward_to_blue/secure/api/service/outlet
  assert_success

  run curl --fail --head --max-time 10 127.0.0.1:7000
  assert_success
}

@test "project requiring credential" {
  skip_if_orchestrator_tests_not_enabled
  skip_if_long_tests_not_enabled

  space_name=$(openssl rand -hex 4)
  project_name=$(openssl rand -hex 4)

  run $OCKAM space create "${space_name}"
  assert_success

  run $OCKAM project create "${space_name}" "${project_name}"
  assert_success

  $OCKAM project information "${project_name}" --output json >"/tmp/${project_name}_project.json"

  export OCKAM_HOME=/tmp/ockam

  run $OCKAM identity create green
  run $OCKAM identity create blue
  green_identifier=$($OCKAM identity show green)
  blue_identifier=$($OCKAM identity show blue)

  run $OCKAM node create green --project "/tmp/${project_name}_project.json" --identity green
  assert_success

  run $OCKAM node create blue --project "/tmp/${project_name}_project.json" --identity blue
  assert_success

  # Blue can't create forwarder as it isn't a member
  run $OCKAM forwarder create blue --at "/project/${project_name}" --to /node/blue
  assert_failure

  # add green as a member
  unset OCKAM_HOME
  run $OCKAM project enroll --member $green_identifier --to "/project/${project_name}/service/authenticator" --attribute role=member
  assert_success

  # Now green can access project' services
  export OCKAM_HOME=/tmp/ockam
  run $OCKAM forwarder create green --at "/project/${project_name}" --to /node/green
  assert_success

  unset OCKAM_HOME
  run $OCKAM project delete "${space_name}" "${project_name}"
  assert_success

  run $OCKAM space delete "${space_name}"
  assert_success
}

@test "project addons - list" {
  skip_if_orchestrator_tests_not_enabled

  run --separate-stderr $OCKAM project addon list --project default

  assert_success
  assert_output --partial "Id: okta"
}

@test "project addons - enable and disable" {
  skip # TODO: wait until cloud has the influxdb and confluent addons enabled
  skip_if_orchestrator_tests_not_enabled

  run --separate-stderr $OCKAM project addon list --project default
  assert_success
  assert_output --partial --regex "Id: okta\n +Enabled: false"
  assert_output --partial --regex "Id: confluent\n +Enabled: false"

  run --separate-stderr $OCKAM project addon enable okta --project default --tenant tenant --client-id client_id --cert cert
  assert_success
  run --separate-stderr $OCKAM project addon enable confluent --project default --bootstrap-server bootstrap-server.confluent:9092 --api-key ApIkEy --api-secret ApIsEcrEt
  assert_success

  run --separate-stderr $OCKAM project addon list --project default
  assert_success
  assert_output --partial --regex "Id: okta\n +Enabled: true"
  assert_output --partial --regex "Id: confluent\n +Enabled: true"

  run --separate-stderr $OCKAM project addon disable --addon okta --project default
  run --separate-stderr $OCKAM project addon disable --addon --project default
  run --separate-stderr $OCKAM project addon disable --addon confluent --project default

  run --separate-stderr $OCKAM project addon list --project default
  assert_success
  assert_output --partial --regex "Id: okta\n +Enabled: false"
  assert_output --partial --regex "Id: confluent\n +Enabled: false"
}

@test "influxdb lease manager" {
  # TODO add more tests cases testing the leases/expiration themselves. This basically just test that the service is there,
  #      responsible, and that a member enrolled on a different ockam install can access it.
  skip_if_orchestrator_tests_not_enabled
  skip_if_influxdb_test_not_enabled

  run $OCKAM project addon configure influxdb --org-id "${INFLUXDB_ORG_ID}" --token "${INFLUXDB_TOKEN}" --endpoint-url "${INFLUXDB_ENDPOINT}" --max-ttl 60 --permissions "${INFLUXDB_PERMISSIONS}"
  assert_success

  sleep 30 #FIXME  workaround, project not yet ready after configuring addon

  $OCKAM project information default --output json >/tmp/project.json

  export OCKAM_HOME=/tmp/ockam
  run $OCKAM identity create m1
  run $OCKAM identity create m2
  run $OCKAM identity create m3

  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  unset OCKAM_HOME
  $OCKAM project enroll --member $m1_identifier --attribute service=sensor
  $OCKAM project enroll --member $m2_identifier --attribute service=web

  export OCKAM_HOME=/tmp/ockam

  # m1 and m2 identity was added by enroller
  run $OCKAM project authenticate --identity m1 --project-path /tmp/project.json
  assert_success
  assert_output --partial $green_identifier

  run $OCKAM project authenticate --identity m2 --project-path /tmp/project.json
  assert_success
  assert_output --partial $green_identifier

  # m1 and m2 can use the lease manager
  run $OCKAM lease --identity m1 --project-path /tmp/project.json create
  assert_success
  run $OCKAM lease --identity m2 --project-path /tmp/project.json create
  assert_success

  # m3 can't
  run $OCKAM lease --identity m3 --project-path /tmp/project.json create
  assert_failure

  unset OCKAM_HOME
  run $OCKAM project addon configure influx-db --org-id "${INFLUXDB_ORG_ID}" --token "${INFLUXDB_TOKEN}" --endpoint-url "${INFLUXDB_ENDPOINT}" --max-ttl 60 --permissions "${INFLUXDB_PERMISSIONS}" --user-access-role '(= subject.service "sensor")'
  assert_success

  sleep 30 #FIXME  workaround, project not yet ready after configuring addon

  export OCKAM_HOME=/tmp/ockam
  # m1 can use the lease manager (it has a service=sensor attribute attested by authority)
  run $OCKAM lease --identity m1 --project-path /tmp/project.json create
  assert_success

  # m2 can't use the  lease manager now (it doesn't have a service=sensor attribute attested by authority)
  run $OCKAM lease --identity m2 --project-path /tmp/project.json create
  assert_failure
}

function skip_if_influxdb_test_not_enabled() {
  # shellcheck disable=SC2031
  if [ -z "${INFLUXDB_TESTS}" ]; then
    skip "INFLUXDB_TESTS are not enabled"
  fi
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

@test "secure-channels with authorized identifiers" {
  run $OCKAM vault delete v1
  run $OCKAM vault delete v2
  run $OCKAM identity delete i1
  run $OCKAM identity delete i2

  run $OCKAM vault create v1
  assert_success
  run $OCKAM identity create i1 --vault v1
  assert_success
  idt1=$($OCKAM identity show i1)

  run $OCKAM vault create v2
  assert_success
  run $OCKAM identity create i2 --vault v2
  assert_success
  idt2=$($OCKAM identity show i2)

  run $OCKAM node create n1 --vault v1 --identity i1
  assert_success
  run $OCKAM node create n2 --vault v1 --identity i1
  assert_success

  run $OCKAM secure-channel-listener create l --at n2 --vault v2 --identity i2 --authorized $idt1
  run bash -c " $OCKAM secure-channel create --from n1 --to /node/n2/service/l --authorized $idt2 \
              | $OCKAM message send hello --from /node/n1 --to -/service/uppercase"
  assert_success
}
