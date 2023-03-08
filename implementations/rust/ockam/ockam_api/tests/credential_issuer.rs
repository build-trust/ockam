use ockam_core::compat::collections::{BTreeMap, HashMap};
use ockam_core::compat::sync::Arc;

use ockam::identity::credential::Timestamp;
use ockam::identity::{identities, secure_channels, AttributesEntry, IdentitiesStorage};
use ockam::route;
use ockam_api::bootstrapped_identities_store::{BootstrapedIdentityStore, PreTrustedIdentities};
use ockam_core::compat::rand::random_string;
use ockam_core::{AllowAll, Result};
use ockam_identity::{
    CredentialsIssuer, CredentialsIssuerClient, SecureChannelListenerOptions, SecureChannelOptions,
};
use ockam_node::Context;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let api_worker_addr = random_string();
    let auth_worker_addr = random_string();

    // create 2 identities to populate the trusted identities
    let identities = identities();
    let auth_identity = identities.identities_creation().create_identity().await?;
    let member_identity = identities.identities_creation().create_identity().await?;

    let now = Timestamp::now().unwrap();

    let pre_trusted = HashMap::from([(
        member_identity.identifier(),
        AttributesEntry::new(
            BTreeMap::from([("attr".to_string(), "value".as_bytes().to_vec())]),
            now,
            None,
            None,
        ),
    )]);

    let boostrapped = BootstrapedIdentityStore::new(
        Arc::new(PreTrustedIdentities::from(pre_trusted)),
        IdentitiesStorage::create(),
    );

    // Now recreate the identities services with the previous vault
    // (so that the authority can verify its signature)
    // and the repository containing the trusted identities
    let identities = identities::builder()
        .with_identities_repository(Arc::new(boostrapped))
        .with_identities_vault(identities.clone().vault())
        .build();
    let secure_channels = secure_channels::builder()
        .with_identities(identities.clone())
        .build();
    let identities_creation = identities.identities_creation();

    // Create the CredentialIssuer:
    secure_channels
        .create_secure_channel_listener(
            ctx,
            &auth_identity,
            &api_worker_addr,
            SecureChannelListenerOptions::new(),
        )
        .await?;
    let auth = CredentialsIssuer::new(
        identities.clone(),
        auth_identity.clone(),
        "project42".into(),
    )
    .await?;
    ctx.start_worker(
        &auth_worker_addr,
        auth,
        AllowAll, // In reality there is ABAC rule here.
        AllowAll,
    )
    .await?;

    // Connect to the API channel from the member:
    let e2a = secure_channels
        .create_secure_channel(
            ctx,
            &member_identity,
            &api_worker_addr,
            SecureChannelOptions::new(),
        )
        .await?;
    // Add the member via the enroller's connection:
    let c = CredentialsIssuerClient::new(route![e2a.address(), &auth_worker_addr], ctx).await?;
    // Get a fresh member credential and verify its validity:
    let credential = c.credential().await?;
    let exported = member_identity.export()?;

    let imported = identities_creation
        .import_identity(&exported)
        .await
        .unwrap();
    let data = identities
        .credentials()
        .verify_credential(&imported.identifier(), &[auth_identity], credential)
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );
    assert_eq!(Some(b"value".as_slice()), data.attributes().get("attr"));
    ctx.stop().await
}
