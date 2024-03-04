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

@test "authority - standalone authority, admin, enrollers, members" {
  port="$(random_port)"

  run "$OCKAM" identity create authority

  # Authority will trust project-admin credentials issued by this other identity (Account Authority)
  run "$OCKAM" identity create account_authority

  run "$OCKAM" identity create admin
  # m1 will be pre-enrolled on authority.  m2 will be added directly, m3 will be added through enrollment token
  # m4 and m5 will be added by a shared enrollment token, m6 won't be added
  run "$OCKAM" identity create m1
  run "$OCKAM" identity create m2
  run "$OCKAM" identity create m3
  run "$OCKAM" identity create m4
  run "$OCKAM" identity create m5
  run "$OCKAM" identity create m7

  account_authority_full=$($OCKAM identity show account_authority --full --encoding hex)
  account_authority_identifier=$($OCKAM identity show account_authority)

  admin_identifier=$($OCKAM identity show admin)
  authority_identity_full=$($OCKAM identity show --full --encoding hex authority)
  m1_identifier=$($OCKAM identity show m1)

  # Create a node for the admin, used as a hack to present the project admin credential to the authority
  port_admin="$(random_port)"
  run_success "$OCKAM" node create admin --tcp-listener-address "127.0.0.1:$port_admin" --identity admin --authority-identity $account_authority_full --expect-cached-credential

  # issue and store project admin credentials for admin
  $OCKAM credential issue --as account_authority --for "$admin_identifier" --attribute project="1"  --encoding hex >"$OCKAM_HOME/admin.cred"
  run_success "$OCKAM" credential store --at admin --issuer "$account_authority_identifier" --credential-path "$OCKAM_HOME/admin.cred"

  # Start the authority node.  We pass a set of pre trusted-identities containing m1' identity identifier
  trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\"} }"
  run_success "$OCKAM" authority create --tcp-listener-address="127.0.0.1:$port" --project-identifier 1 --trusted-identities "$trusted" --no-direct-authentication --account-authority $account_authority_full
  sleep 2 # wait for authority to start TCP listener

  # Make the admin present its project admin credential to the authority
  run_success "$OCKAM" secure-channel create --from admin --to "/node/authority/service/api" --identity admin


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


  run_success $OCKAM project import --project-file $OCKAM_HOME/project.json

  run_success "$OCKAM" project enroll --identity admin
  assert_output --partial '"ockam-relay": "*"'
  assert_output --partial "$admin_identifier"

  # m1 is a member (its on the set of pre-trusted identifiers) so it can get it's own credential
  run_success "$OCKAM" project enroll --identity m1
  assert_output --partial "sample_val"

  # admin can enroll new members, because it has presented a project-admin credential to the authority
  # and that is still valid (even if it doesn't present it again here)
  token1=$($OCKAM project ticket --identity admin --attribute sample_attr=m2_member)
  run_success "$OCKAM" project enroll $token1 --identity m2
  assert_output --partial "m2_member"

  token2=$($OCKAM project ticket --identity admin --usage-count 2 --attribute sample_attr=members_group)
  run_success "$OCKAM" project enroll $token2 --identity m3
  assert_output --partial "members_group"

  run_success "$OCKAM" project enroll $token2 --identity m4
  assert_output --partial "members_group"

  # admin can enroll new enrollers
  token3=$($OCKAM project ticket --identity admin --enroller)
  run_success "$OCKAM" project enroll $token3 --identity m7
  assert_output --partial "enroller"

  # New enroller can enroll members
  run_success "$OCKAM" project ticket --identity m7

  # For now, it can enroll new enrollers as well.
  run_success "$OCKAM" project ticket --enroller --identity m7

  run "$OCKAM" project enroll $token2 --identity m5
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
  trusted="{\"$enroller_identifier\": {\"ockam-role\": \"enroller\"}}"
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
