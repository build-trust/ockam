#!/bin/bash

# ==== SETUP
setup() {
  load load/base.bash
  load_bats_ext
  setup_home_dir
}

teardown() {
  teardown_home_dir
}

# ====== TESTS

@test "secure-channels - create a secure channel between two nodes explicitly providing a credential authorized by a third identity" {
    msg=$(random_str)

    # Create Two identities
    $OCKAM identity create i1
    $OCKAM identity show i1 > i1.id

    $OCKAM identity create i2
    $OCKAM identity show i2 > i2.id

    # Create an identity that both i1, and i2 can trust
    $OCKAM identity create identity_authority
    $OCKAM identity show identity_authority > ia.id
    $OCKAM identity show identity_authority --full --encoding hex > authority

    # issue and store credentials for i1
    $OCKAM credential issue --as identity_authority --for $(cat i1.id) --attribute city="New York" --encoding hex > i1.cred
    $OCKAM credential store i1-cred --issuer $(cat ia.id) --credential-path i1.cred

    # issue credential for i2
    $OCKAM credential issue --as identity_authority --for $(cat i2.id) --attribute city="Dallas" --encoding hex > i2.cred
    $OCKAM credential store i2-cred --issuer $(cat ia.id) --credential-path i2.cred

    # Create a node that trust identity_authority as a credential authority
    $OCKAM node create n1 --authority-identity $(cat authority)

    # TODO: This next command could omit the preset credential, if we update
    #       the `secure-channel create` command to be able to use
    #       CredentialExchangeMode::OneWay instead of Mutual
    # Create another node that trust and has a preset credential
    $OCKAM node create n2 --authority-identity $(cat authority) --identity i2 --credential i2-cred


    # Create a secure channel between n1 and n2
    # n1 will present the credential provided within this command
    # n2 will present the cerdential preset when created
    run --separate-stderr $OCKAM secure-channel create --from /node/n1 --to /node/n2/service/api --credential i1-credential
    assert_success
}
