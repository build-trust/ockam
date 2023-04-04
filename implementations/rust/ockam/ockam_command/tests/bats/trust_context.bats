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
        |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    assert_success
}

@test "trust context - trust context with an id only; ABAC rules are applied" {
    #  run "$OCKAM" node create n1 --identity m1 --trust-context { "id": "trust_context_id", "authority": null }

    #  run "$OCKAM" node create n2 --identity m2 --trust-context { "id": "trust_context_id", "authority": null }

    #  run "$OCKAM" node create n3 --identity m2 --trust-context { "id": "wrong_trust_context_id", "authority": null }

    #  run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
    #     |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo

    # # assert success

    # pr secure-channel create --from /node/n1 --to /node/n3/service/api \
    #     |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    # # assert failure
}

@test "trust context - trust context with an identity authority; Credential Exchange is performed" {
    #  run "$OCKAM" node create n1 --identity m1 --trust-context { "id": "trust_context_id", "authority": "trust_context_authority" }

    #  run "$OCKAM" node create n2 --identity m2 --trust-context { "id": "trust_context_id", "authority": "trust_context_authority" }

    #  run "$OCKAM" node create n3 --identity m2 --trust-context { "id": "trust_context_id", "authority": "wrong_trust_context_authority" }

    #  run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
    #     |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo

    # # assert success

    #  run "$OCKAM" secure-channel create --from /node/n1 --to /node/n3/service/api \
    #     |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    # # assert failure
}

@test "trust context - trust context with an id and authority using orchestrator; orchestrator enrollment and connection is performed, orchestrator" {
    skip_if_orchestrator_tests_not_enabled
    load_orchestrator_data
    run "$OCKAM" project information --as-trust-context > trust_context.json

    run "$OCKAM" identity create m1
    run "$OCKAM" project enroll > m1.token
    run "$OCKAM" project authenticate --identity m1 --trust-context ./trust_context.json --token $(cat m1.token)

    run "$OCKAM" identity create m2
    run "$OCKAM" project enroll > m2.token
    run "$OCKAM" project authenticate --identity m2 --trust-context ./trust_context.json --token $(cat m2.token)

    run "$OCKAM" node create n1 --identity m1 --trust-context ./trust_context.json

    run "$OCKAM" node create n2 --identity m2 --trust-context ./trust_context.json

    run "$OCKAM" secure-channel create --from /node/n1 --to /node/n2/service/api \
    |  run "$OCKAM" message send hello --from /node/n1 --to -/service/echo
    assert_success
}