#!/bin/bash

# ===== SETUP

setup() {
  load load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ===== TESTS
@test "authority - an authority node must be shown as UP even if its tcp listener cannot be accessed" {
  port="$(random_port)"

  run_success "$OCKAM" identity create authority
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)
  trusted="{}"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted"
  run_success "$OCKAM" node show authority
  assert_output --partial "\"is_up\": true"
}

@test "authority - an authority identity is created by default for the authority node" {
  port="$(random_port)"

  trusted="{}"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted"
  run_success "$OCKAM" identity show authority
}

@test "authority - an authority identity is created by default for the authority node - with a given name" {
  port="$(random_port)"

  trusted="{}"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted" --identity ockam
  run_success "$OCKAM" identity show ockam
}

@test "authority - standalone authority, enrollers, members" {
  port="$(random_port)"

  run "$OCKAM" identity create authority
  run "$OCKAM" identity create enroller
  # m1 will be pre-enrolled on authority.  m2 will be added directly, m3 will be added through enrollment token
  # m4 and m5 will be added by a shared enrollment token, m6 won't be added
  run "$OCKAM" identity create m1
  run "$OCKAM" identity create m2
  run "$OCKAM" identity create m3
  run "$OCKAM" identity create m4
  run "$OCKAM" identity create m5
  run "$OCKAM" identity create m6

  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)
  m1_identifier=$($OCKAM identity show m1)
  m2_identifier=$($OCKAM identity show m2)

  # Start the authority node.  We pass a set of pre trusted-identities containing m1' identity identifier
  # For the first test we start the node with no direct authentication service nor token enrollment
  trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\", \"project_id\" : \"1\", \"trust_context_id\" : \"1\"}, \"$enroller_identifier\": {\"project_id\": \"1\", \"trust_context_id\": \"1\", \"ockam-role\": \"enroller\"}}"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted" --no-direct-authentication --no-token-enrollment
  sleep 1 # wait for authority to start TCP listener

  cat <<EOF >>"$OCKAM_HOME/project.json"
{
  "id": "1",
  "name": "default",
  "space_name": "together-porgy",
  "access_route": "/dnsaddr/127.0.0.1/tcp/4000/service/api",
  "users": [],
  "space_id": "1",
  "identity": "I6c20e814b56579306f55c64e8747e6c1b4a53d9aa1b2c3d4e5f6a6b5c4d3e2f1",
  "authority_access_route": "/dnsaddr/127.0.0.1/tcp/$port/service/api",
  "authority_identity": "$authority_identity_full",
  "version": "605c4632ded93eb17edeeef31fa3860db225b3ab-2023-12-05",
  "running": false,
  "operation_id": null,
  "user_roles": []
}
EOF

  run_success bash -c "$OCKAM project import --project-file $OCKAM_HOME/project.json"

  # m1 is a member (its on the set of pre-trusted identifiers) so it can get it's own credential
  run_success "$OCKAM" project enroll --identity m1
  assert_output --partial "sample_val"

  echo "$trusted" >"$OCKAM_HOME/trusted-anchors.json"
  # Restart the authority node with a trusted identities file and check that m1 can still enroll
  run_success "$OCKAM" node delete authority --yes
  run_success "$OCKAM" authority create --tcp-listener-address=127.0.0.1:$port --project-identifier 1 --reload-from-trusted-identities-file "$OCKAM_HOME/trusted-anchors.json"
  sleep 1 # wait for authority to start TCP listener

  run_success "$OCKAM" project ticket --identity enroller --member $m2_identifier --attribute sample_attr=m2_member

  run_success "$OCKAM" project enroll --force --identity m2
  assert_output --partial "m2_member"

  token1=$($OCKAM project ticket --identity enroller --attribute sample_attr=m3_member)
  run_success "$OCKAM" project enroll --force $token1 --identity m3
  assert_output --partial "m3_member"

  token2=$($OCKAM project ticket --identity enroller --usage-count 2 --attribute sample_attr=members_group)
  run_success "$OCKAM" project enroll --force $token2 --identity m4
  assert_output --partial "members_group"

  run_success "$OCKAM" project enroll --force $token2 --identity m5
  assert_output --partial "members_group"

  run "$OCKAM" project enroll --force $token2 --identity m6
  assert_failure
}

@test "authority - enrollment ticket ttl" {
  port="$(random_port)"

  run "$OCKAM" identity create authority
  run "$OCKAM" identity create enroller
  #m3 will be added through enrollment token
  run "$OCKAM" identity create m3

  enroller_identifier=$($OCKAM identity show enroller)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)

  # Start the authority node.
  trusted="{\"$enroller_identifier\": {\"project_id\": \"1\", \"trust_context_id\": \"1\", \"ockam-role\": \"enroller\"}}"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted"
  sleep 1 # wait for authority to start TCP listener

  cat <<EOF >>"$OCKAM_HOME/project.json"
{
  "id": "1",
  "name": "default",
  "space_name": "together-porgy",
  "access_route": "/dnsaddr/127.0.0.1/tcp/4000/service/api",
  "users": [],
  "space_id": "1",
  "identity": "I6c20e814b56579306f55c64e8747e6c1b4a53d9aa1b2c3d4e5f6a6b5c4d3e2f1",
  "authority_access_route": "/dnsaddr/127.0.0.1/tcp/$port/service/api",
  "authority_identity": "$authority_identity_full",
  "version": "605c4632ded93eb17edeeef31fa3860db225b3ab-2023-12-05",
  "running": false,
  "operation_id": null,
  "user_roles": []
}
EOF

  run_success bash -c "$OCKAM project import --project-file $OCKAM_HOME/project.json"

  # Enrollment ticket expired by the time it's used
  token=$($OCKAM project ticket --identity enroller --attribute sample_attr=m3_member --expires-in 1s)
  sleep 2
  run "$OCKAM" project enroll $token --identity m3
  assert_failure

  # Enrollment ticket with enough ttl
  token=$($OCKAM project ticket --identity enroller --attribute sample_attr=m3_member --expires-in 30s)
  run_success "$OCKAM" project enroll $token --identity m3
  assert_output --partial "m3_member"
}
