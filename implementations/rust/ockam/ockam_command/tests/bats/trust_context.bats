#!/bin/bash

# ===== SETUP

setup() {
    load load/base.bash
    load load/orchestrator.bash
    load_bats_ext
    setup_home_dir
}

teardown() {
    teardown_home_dir
}

# ===== TESTS

@test "trust context - no trust context; everything is accepted" {
    run "$OCKAM" identity create m1
    run "$OCKAM" node create n1 --identity m1

    run "$OCKAM" identity create m2
    run "$OCKAM" node create n2 --identity m2

    run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
        | "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    assert_success
}

@test "trust context - trust context with an id only; ABAC rules are applied" {
    echo "{
        \"id\": \"1\"
    }" > ./trust_context.json

    run "$OCKAM" identity create m1

    m1_identifier=$(run "$OCKAM"  identity show m1)
    trusted="{\"$m1_identifier\": {\"sample_attr\": \"sample_val\", \"project_id\" : \"1\", \"trust_context_id\" : \"1\"}}"

    run "$OCKAM" node create n1 --identity m1

    run "$OCKAM" node create n2  --trust-context ./trust_context.json --trusted-identities "$trusted"

    run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
        | "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    assert_success

    run "$OCKAM" message send hello --from /node/n1 --to /node/n2/service/echo
    assert_failure
}

@test "trust context - trust context with an identity authority; Credential Exchange is performed" {
    # Create two identities
    run "$OCKAM" identity create i1
    i1_id=$($OCKAM identity show i1 --full --encoding hex)

    run "$OCKAM" identity create i2
    i2_id=$($OCKAM identity show i2 --full --encoding hex)

    # Create an identity that both i1, and i2 can trust
    run "$OCKAM" identity create identity_authority
    authority_id=$($OCKAM identity show identity_authority --full --encoding hex)

    # issue and store credentials for i1
    $OCKAM credential issue --as identity_authority --for $i1_id --attribute city="New York" --encoding hex > i1.cred
    run "$OCKAM" credential store i1-cred --issuer $authority_id --credential-path i1.cred
    $OCKAM credential show i1-cred --as-trust-context > ./i1-trust-context.json

    # issue and store credential for i2
    $OCKAM credential issue --as identity_authority --for $i2_id --attribute city="Boston" --encoding hex > i2.cred
    run "$OCKAM" credential store i2-cred --issuer $authority_id --credential-path i2.cred
    $OCKAM credential show i2-cred --as-trust-context > i2-trust-context.json

    # Create a node for i1 that trust identity_authority as a credential authority
    run "$OCKAM" node create n1 --identity i1 --trust-context i1-trust-context.json

    # Create a node for i2 that trust identity_authority as a credential authority
    run "$OCKAM" node create n2 --identity i2 --trust-context i2-trust-context.json

    run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api
    assert_success
}

@test "trust context - trust context with an id and authority using orchestrator; orchestrator enrollment and connection is performed, orchestrator" {
    skip_if_orchestrator_tests_not_enabled
    load_orchestrator_data
    $OCKAM project information --as-trust-context > trust_context.json

    run "$OCKAM" identity create m1
    $OCKAM project enroll > m1.token
    run "$OCKAM" project authenticate --identity m1 --trust-context ./trust_context.json --token $(cat m1.token)

    run "$OCKAM" identity create m2
    $OCKAM project enroll > m2.token
    run "$OCKAM" project authenticate --identity m2 --trust-context ./trust_context.json --token $(cat m2.token)

    run "$OCKAM" node create n1 --identity m1 --trust-context ./trust_context.json

    run "$OCKAM" node create n2 --identity m2 --trust-context ./trust_context.json

    run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
        | "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    assert_success
}
