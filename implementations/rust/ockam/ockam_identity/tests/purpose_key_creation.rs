use ockam_core::Result;
use ockam_identity::identities;
use ockam_vault::{SigningKeyType, VerifyingPublicKey};

#[tokio::test]
async fn create_default_purpose_keys() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation.create_identity().await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_credential_purpose_key(identity.identifier())
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_secure_channel_purpose_key(identity.identifier())
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn create_custom_type() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation.create_identity().await?;

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .credential_purpose_key_builder(identity.identifier())
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;

    let purpose_key = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await?;

    let key_id = purpose_key.key();
    let public_key = identities
        .vault()
        .credential_vault
        .get_verifying_public_key(key_id)
        .await?;
    matches!(public_key, VerifyingPublicKey::ECDSASHA256CurveP256(_));

    Ok(())
}

#[tokio::test]
async fn create_with_p256_identity() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation
        .identity_builder()
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_credential_purpose_key(identity.identifier())
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_secure_channel_purpose_key(identity.identifier())
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_secure_channel_purpose_key(identity.identifier())
        .await;
    assert!(res.is_ok());

    Ok(())
}

#[tokio::test]
async fn create_with_rotated_identity() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation.create_identity().await?;

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_credential_purpose_key(identity.identifier())
        .await?;

    identities_creation
        .rotate_identity(identity.identifier())
        .await?;

    // We currently do not verify Purpose Keys issued by an older version of identity
    let res = purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_credential_purpose_key(identity.identifier())
        .await?;

    purpose_keys
        .purpose_keys_creation()
        .get_credential_purpose_key(identity.identifier())
        .await?;

    Ok(())
}
