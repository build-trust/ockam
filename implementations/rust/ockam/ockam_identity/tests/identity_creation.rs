use core::str::FromStr;

use ockam_core::Result;
use ockam_identity::identities;
use ockam_identity::models::Identifier;
use ockam_vault::SigningKeyType;

#[tokio::test]
async fn create_and_retrieve() -> Result<()> {
    let identities = identities().await?;
    let identities_creation = identities.identities_creation();
    let identities_verification = identities.identities_verification();
    let identities_keys = identities.identities_keys();

    let identifier = identities_creation.create_identity().await?;
    let actual = identities_verification.get_identity(&identifier).await?;

    assert_eq!(
        actual.identifier(),
        &identifier,
        "the identity can be retrieved from the repository"
    );

    let actual = identities_verification
        .get_change_history(&identifier)
        .await;
    assert!(actual.is_ok());

    let another_identifier =
        Identifier::from_str("Ie92f183eb4c324804ef4d62962dea94cf095a265a1b2c3d4e5f6a6b5c4d3e2f1")?;
    let missing = identities_verification
        .get_identity(&another_identifier)
        .await
        .ok();
    assert_eq!(missing, None, "a missing identity returns an error");

    let identity = identities_verification.get_identity(&identifier).await?;
    let root_key = identities_keys.get_secret_key(&identity).await;
    assert!(root_key.is_ok(), "there is a key for the created identity");

    Ok(())
}

#[tokio::test]
async fn create_p256() -> Result<()> {
    let identities = identities().await?;
    let identities_creation = identities.identities_creation();
    let identities_verification = identities.identities_verification();
    let identities_keys = identities.identities_keys();

    let identifier = identities_creation
        .identity_builder()
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;
    let actual = identities_verification.get_identity(&identifier).await?;

    assert_eq!(
        actual.identifier(),
        &identifier,
        "the identity can be retrieved from the repository"
    );

    let root_key = identities_keys.get_secret_key(&actual).await;
    assert!(root_key.is_ok(), "there is a key for the created identity");

    Ok(())
}
