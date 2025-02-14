#!/bin/bash

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

@test "node control api - relay node resolution" {
  ticket=$($OCKAM project ticket \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend \
    --relay red)
  api_port="$(random_port)"
  frontend_node_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --tcp-listener-address "127.0.0.1:${frontend_node_port}" \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\", \
      \"node_resolution\":\"relay\" \
    }}}"
  wait_for_port $api_port

  # create a node with a relay in the api node
  run_success "$OCKAM" node create red \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{ \"backend\":true,\"node_resolution\":\"direct-connection\" \
    }}}"
  run_success "$OCKAM" relay create --to red --at "/ip4/127.0.0.1/tcp/${frontend_node_port}/secure/api" red

  # verify that it can be accessed via control node api
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o inlet-list.json \
    "http://localhost:${api_port}/red/tcp-inlets"
  run_success cat inlet-list.json
  assert_output "[]"
}

@test "node control api - direct connection node resolution" {
  ticket=$($OCKAM project ticket \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend)

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  api_port="$(random_port)"
  expected_connection_port="$(random_port)"
  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":false, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\", \
      \"node_resolution\":\"direct-connection\", \
      \"connection_node_port\":${expected_connection_port} \
    }}}"
  wait_for_port $api_port

  # create a node with listening on the expected port
  run_success "$OCKAM" node create \
    --tcp-listener-address "127.0.0.1:${expected_connection_port}" \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{ \"backend\":true,\"node_resolution\":\"direct-connection\" \
    }}}"
  wait_for_port ${expected_connection_port}

  # verify that it can be accessed via control node api
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o inlet-list.json \
    "http://localhost:${api_port}/localhost/tcp-inlets"
  run_success cat inlet-list.json
  assert_output "[]"
}

@test "node control api - portals" {
  ticket=$($OCKAM project ticket \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend)
  api_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\" \
    }}}"
  wait_for_port $api_port

  # create outlet
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"kind\":\"regular\",\"address\":\"my-outlet\",\"to\":{\"hostname\":\"localhost\", \"port\":$PYTHON_SERVER_PORT}}" \
    -o outlet-creation.json \
    "http://localhost:${api_port}/self/tcp-outlets"
  run_success sh -c "cat outlet-creation.json | jq -rc .address"
  assert_output "my-outlet"

  # verify that the outlet can be retrieved
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o outlet.json \
    "http://localhost:${api_port}/self/tcp-outlets/my-outlet"
  run_success sh -c "cat outlet.json | jq -rc .address"
  assert_output "my-outlet"

  # verify that the outlet is listed
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o outlet-list.json \
    "http://localhost:${api_port}/self/tcp-outlets"
  run_success sh -c "cat outlet-list.json | jq -rc .[0].address"
  assert_output "my-outlet"

  # create inlet
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"from\":{\"hostname\":\"127.0.0.1\",\"port\":0},\"kind\":\"regular\",\"name\":\"my-inlet\",\"to\":\"/secure/api/service/my-outlet\"}" \
    -o inlet-creation.json \
    "http://localhost:${api_port}/self/tcp-inlets"
  inlet_port=$(cat inlet-creation.json | jq -rc '."bind-address".port')
  wait_for_port $inlet_port

  # verify that the inlet can be retrieved
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o inlet.json \
    "http://localhost:${api_port}/self/tcp-inlets/my-inlet"
  run_success sh -c "cat inlet.json | jq -rc .name"
  assert_output "my-inlet"

  # verify that the inlet is listed
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o inlet-list.json \
    "http://localhost:${api_port}/self/tcp-inlets"
  run_success sh -c "cat inlet-list.json | jq -rc .[0].name"
  assert_output "my-inlet"

  # verify that the portal is working
  run_success curl -sfI --retry-all-errors --retry-delay 5 --retry 10 -m 5 "127.0.0.1:$inlet_port"

  # delete the outlet
  run_success curl -vf \
    -X DELETE \
    -H 'Authorization: Bearer token' \
    "http://localhost:${api_port}/self/tcp-outlets/my-outlet"

  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o outlet-list.json \
    "http://localhost:${api_port}/self/tcp-outlets"
  run_success sh -c "cat outlet-list.json | jq -rc ."
  assert_output "[]"

  # delete the inlet
  run_success curl -vf \
    -X DELETE \
    -H 'Authorization: Bearer token' \
    "http://localhost:${api_port}/self/tcp-inlets/my-inlet"

  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o inlet-list.json \
    "http://localhost:${api_port}/self/tcp-inlets"
  run_success sh -c "cat inlet-list.json | jq -rc ."
  assert_output "[]"
}

@test "node control api - relay" {
  ticket=$($OCKAM project ticket \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend \
    --relay "my-address")
  api_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\" \
    }}}"
  wait_for_port $api_port

  # create relay
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"address\":\"my-address\",\"name\":\"my-relay\",\"to\":\"/project/default\"}" \
    -o relay-creation.json \
    "http://localhost:${api_port}/self/relays"
  run_success sh -c "cat relay-creation.json | jq -rc .name"
  assert_output "my-relay"

  # retrieve the relay status until it's ready
  while true; do
    run_success curl -vf \
      -H 'Authorization: Bearer token' \
      -o relay.json \
      "http://localhost:${api_port}/self/relays/my-relay"
    run_success sh -c "cat relay.json | jq -rc .name"
    assert_output "my-relay"

    if [ "$(cat relay.json | jq -rc .status)" != "up" ]; then
      sleep 0.2
      continue
    fi

    run_success sh -c "cat relay.json | jq -rc '.\"remote-address\"'"
    assert_output "forward_to_my-address"
    break
  done

  # verify that the relay is listed
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o relay-list.json \
    "http://localhost:${api_port}/self/relays"
  run_success sh -c "cat relay-list.json | jq -rc .[0].name"
  assert_output "my-relay"

  # verify that the relay is working
  run_success "$OCKAM" message send --to /project/default/service/forward_to_my-address/secure/api/service/echo "hello"
  assert_output "hello"

  # delete the relay
  run_success curl -vf \
    -X DELETE \
    -H 'Authorization: Bearer token' \
    "http://localhost:${api_port}/self/relays/my-relay"

  # verify that the relay is deleted
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o relay-list.json \
    "http://localhost:${api_port}/self/relays"
  run_success sh -c "cat relay-list.json | jq -rc ."
  assert_output "[]"
}

@test "node control api - ticket" {
  ticket=$($OCKAM project ticket \
    --enroller \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend)
  api_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\" \
    }}}"
  wait_for_port $api_port

  # create ticket
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"attributes\":{\"ockam-role\":\"member\",\"my-attribute\":\"my-value\"}}" \
    -o ticket-creation.json \
    "http://localhost:${api_port}/self/tickets"
  ticket=$(cat ticket-creation.json | jq -rc .encoded)

  # use the ticket with a dedicated identity
  run_success "$OCKAM" identity create "my-identity"
  identifier=$($OCKAM identity show "my-identity" --output json | jq -rc .identifier)

  # enroll the identity with the ticket
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"ticket\":\"$ticket\",\"identity\":\"$identifier\"}" \
    -o enrollment.json \
    "http://localhost:${api_port}/self/tickets/enroll"
  run_success sh -c "cat enrollment.json | jq -rc .identity"
}

@test "node control api - local authority ticket" {
  ticket=$($OCKAM project ticket \
    --enroller \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend)
  api_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\" \
    }}}"
  wait_for_port $api_port

  default_identifier=$($OCKAM identity show --output json | jq -rc .identifier)
  default_identity_change_history=$($OCKAM identity show --full --encoding hex)

  run_success "$OCKAM" identity create "authority-identity"
  authority_identity_change_history=$($OCKAM identity show "authority-identity" --full --encoding hex)

  trusted_identities="{\"${default_identifier}\": {\"ockam-role\": \"enroller\"} }"
  authority_port="$(random_port)"
  run_success "$OCKAM" authority create --identity "authority-identity" --tcp-listener-address "127.0.0.1:${authority_port}" --project-identifier "$default_identifier" --trusted-identities "${trusted_identities}"
  wait_for_port $authority_port

  # create ticket
  # request is a bit too complex to inline, let's convert json to an escaped string
  request=$(
    cat <<EOF | jq -rc .
{
  "attributes": {
    "ockam-role": "member",
    "my-attribute": "my-value"
  },
  "project": {
    "provided": {
      "authority_route": "/ip4/127.0.0.1/tcp/$authority_port/service/api",
      "authority_change_history": "$authority_identity_change_history",
      "project_route": "ip4/127.0.0.1/tcp/$api_port/service/api",
      "project_change_history": "$default_identity_change_history"
    }
  }
}
EOF
  )

  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "${request}" \
    -o ticket-creation.json \
    "http://localhost:${api_port}/self/tickets"
  ticket=$(cat ticket-creation.json | jq -rc .encoded)

  # use the ticket with a dedicated identity
  run_success "$OCKAM" identity create "my-identity"
  identifier=$($OCKAM identity show "my-identity" --output json | jq -rc .identifier)

  # enroll the identity with the ticket
  run_success curl -vf \
    -X POST \
    -H 'Authorization: Bearer token' \
    -d "{\"ticket\":\"$ticket\",\"identity\":\"$identifier\"}" \
    -o enrollment.json \
    "http://localhost:${api_port}/self/tickets/enroll"
  run_success sh -c "cat enrollment.json | jq -rc .identity"
}

@test "node control api - authority members" {
  ticket=$($OCKAM project ticket \
    --enroller \
    --attribute node_control_api_backend \
    --attribute node_control_api_frontend)
  api_port="$(random_port)"

  setup_home_dir
  run_success "$OCKAM" project enroll "${ticket}"

  run_success "$OCKAM" node create \
    --launch-configuration \
    "{\"start_default_services\": true, \"startup_services\":{ \
      \"control_api\":{\"authentication_token\": \"token\", \"backend\":true, \"frontend\":true, \"http_bind_address\":\"127.0.0.1:${api_port}\" \
    }}}"
  wait_for_port $api_port

  # add member
  run_success curl -vf \
    -X PUT \
    -H 'Authorization: Bearer token' \
    -d "{\"attributes\":{\"ockam-role\":\"member\",\"my-attribute\":\"my-value\"}}" \
    "http://localhost:${api_port}/self/authority-members/Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48"

  # get member
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o get-member.json \
    "http://localhost:${api_port}/self/authority-members/Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48"
  run_success sh -c "cat get-member.json | jq -rc .attributes.\\\"my-attribute\\\""
  assert_output "my-value"
  run_success sh -c "cat get-member.json | jq -rc .identity"
  assert_output "Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48"

  # list members
  run_success curl -vf \
    -H 'Authorization: Bearer token' \
    -o list-members.json \
    "http://localhost:${api_port}/self/authority-members"
  # check that the identity is listed as a member by making a search with jq
  run_success sh -c "cat list-members.json | jq -rc '.[] | select(.identity == \"Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48\").attributes.\"my-attribute\"'"
  assert_output "my-value"

  # delete member
  run_success curl -vf \
    -X DELETE \
    -H 'Authorization: Bearer token' \
    "http://localhost:${api_port}/self/authority-members/Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48"

  # verify that the member is deleted
  run_failure curl -vf \
    -H 'Authorization: Bearer token' \
    "http://localhost:${api_port}/self/authority-members/Ia641901932d24b8a63b51cb78ebe099b3341dcbd6aaa202cc36868bec72bbd48"
}
