use core::str::FromStr;
use ockam_core::Result;
use ockam_identity::models::Identifier;
use ockam_identity::{identities, Identity};
use ockam_vault::SigningKeyType;

#[tokio::test]
async fn create_and_retrieve() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let repository = identities.repository();
    let identities_keys = identities.identities_keys();

    let identity = identities_creation.create_identity().await?;
    let actual = repository.get_identity(identity.identifier()).await?;

    let actual = Identity::import_from_change_history(
        Some(identity.identifier()),
        actual,
        identities.vault().verifying_vault,
    )
    .await?;
    assert_eq!(
        actual, identity,
        "the identity can be retrieved from the repository"
    );

    let actual = repository.retrieve_identity(identity.identifier()).await?;
    assert!(actual.is_some());
    let actual = Identity::import_from_change_history(
        Some(identity.identifier()),
        actual.unwrap(),
        identities.vault().verifying_vault,
    )
    .await?;
    assert_eq!(
        actual, identity,
        "the identity can be retrieved from the repository as an Option"
    );

    let another_identifier = Identifier::from_str("Ie92f183eb4c324804ef4d62962dea94cf095a265")?;
    let missing = repository.retrieve_identity(&another_identifier).await?;
    assert_eq!(missing, None, "a missing identity returns None");

    let root_key = identities_keys.get_secret_key(&identity).await;
    assert!(root_key.is_ok(), "there is a key for the created identity");

    Ok(())
}

#[tokio::test]
async fn create_p256() -> Result<()> {
    let identities = identities();
    let identities_creation = identities.identities_creation();
    let repository = identities.repository();
    let identities_keys = identities.identities_keys();

    let identity = identities_creation
        .identity_builder()
        .with_random_key(SigningKeyType::ECDSASHA256CurveP256)
        .build()
        .await?;
    let actual = repository.get_identity(identity.identifier()).await?;

    let actual = Identity::import_from_change_history(
        Some(identity.identifier()),
        actual,
        identities.vault().verifying_vault,
    )
    .await?;
    assert_eq!(
        actual, identity,
        "the identity can be retrieved from the repository"
    );

    let root_key = identities_keys.get_secret_key(&identity).await;
    assert!(root_key.is_ok(), "there is a key for the created identity");

    Ok(())
}
