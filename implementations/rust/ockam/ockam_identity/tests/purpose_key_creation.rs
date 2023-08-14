use ockam_core::Result;
use ockam_identity::{identities, Purpose};
use ockam_vault::SecretType;

#[tokio::test]
async fn create_default_purpose_keys() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation.create_identity().await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_err());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
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
        .purpose_key_builder(identity.identifier(), Purpose::Credentials)
        .with_random_key(SecretType::NistP256)
        .build()
        .await?;

    let purpose_key = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    let key_id = purpose_key.key_id();
    let stype = identities
        .vault()
        .credential_vault
        .get_public_key(key_id)
        .await?
        .stype();
    assert_eq!(stype, SecretType::NistP256);

    let res = purpose_keys
        .purpose_keys_creation()
        .purpose_key_builder(identity.identifier(), Purpose::SecureChannel)
        .with_random_key(SecretType::NistP256)
        .build()
        .await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn create_with_p256_identity() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let purpose_keys = identities.purpose_keys();

    let identity = identities_creation
        .identity_builder()
        .with_random_key(SecretType::NistP256)
        .build()
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_err());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_purpose_key(identity.identifier(), Purpose::SecureChannel)
        .await?;

    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_ok());
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::SecureChannel)
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
        .create_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    identities_creation
        .rotate_identity(identity.identifier())
        .await?;

    // We currently do not verify Purpose Keys issued by an older version of identity
    let res = purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await;
    assert!(res.is_err());

    let _purpose_key = purpose_keys
        .purpose_keys_creation()
        .create_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    purpose_keys
        .purpose_keys_creation()
        .get_purpose_key(identity.identifier(), Purpose::Credentials)
        .await?;

    Ok(())
}
