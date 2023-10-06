use minicbor::bytes::ByteSlice;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::utils::now;
use ockam::identity::{SecureChannelListenerOptions, SecureChannelOptions, SecureChannels};
use ockam::route;
use ockam_api::authenticator::credentials_issuer::CredentialsIssuer;
use ockam_api::authenticator::{InMemoryMembersStorage, Member, MembersStorage};
use ockam_core::api::Request;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Result};
use ockam_node::api::Client;
use ockam_node::Context;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let api_worker_addr = Address::random_local();
    let auth_worker_addr = Address::random_local();

    let secure_channels = SecureChannels::builder().build();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();

    // create 2 identities to populate the trusted identities
    let auth_identity = identities_creation.create_identity().await?;
    let member_identity = identities_creation.create_identity().await?;

    let now = now().unwrap();

    let members_storage = Arc::new(InMemoryMembersStorage::new());

    let member = Member::new(
        member_identity.identifier().clone(),
        BTreeMap::from([(b"attr".to_vec(), b"value".to_vec())]),
        None,
        now,
        true,
    );
    members_storage.add_member(member).await?;

    // Create the CredentialIssuer:
    let options = SecureChannelListenerOptions::new();
    let sc_flow_control_id = options.spawner_flow_control_id();
    secure_channels
        .create_secure_channel_listener(
            ctx,
            auth_identity.identifier(),
            api_worker_addr.clone(),
            options,
        )
        .await?;
    ctx.flow_controls()
        .add_consumer(auth_worker_addr.clone(), &sc_flow_control_id);
    let auth = CredentialsIssuer::new(
        members_storage,
        identities.credentials(),
        auth_identity.identifier(),
    );
    ctx.start_worker(auth_worker_addr.clone(), auth).await?;

    // Connect to the API channel from the member:
    let e2a = secure_channels
        .create_secure_channel(
            ctx,
            member_identity.identifier(),
            api_worker_addr,
            SecureChannelOptions::new(),
        )
        .await?;
    // Add the member via the enroller's connection:
    // Get a fresh member credential and verify its validity:
    let client = Client::new(&route![e2a, auth_worker_addr], None);
    let credential: CredentialAndPurposeKey =
        client.ask(ctx, Request::post("/")).await?.success()?;

    let exported = member_identity.export()?;

    let imported = identities_creation
        .import(Some(member_identity.identifier()), &exported)
        .await
        .unwrap();
    let data = identities
        .credentials()
        .credentials_verification()
        .verify_credential(
            Some(imported.identifier()),
            &[auth_identity.identifier().clone()],
            &credential,
        )
        .await?;
    assert_eq!(
        Some(&b"value".to_vec().into()),
        data.credential_data
            .subject_attributes
            .map
            .get::<ByteSlice>(b"attr".as_slice().into())
    );
    ctx.stop().await
}
