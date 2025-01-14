use minicbor::bytes::ByteSlice;
use ockam::identity::models::CredentialAndPurposeKey;
use ockam::identity::utils::now;
use ockam::identity::{identities, SecureChannelSqlxDatabase};
use ockam::identity::{
    Identities, SecureChannelListenerOptions, SecureChannelOptions, SecureChannels,
};
use ockam::route;
use ockam_api::authenticator::credential_issuer::CredentialIssuerWorker;
use ockam_api::authenticator::{
    AuthorityMembersRepository, AuthorityMembersSqlxDatabase, PreTrustedIdentity,
};
use ockam_core::api::Request;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Result};
use ockam_node::api::Client;
use ockam_node::Context;
use std::sync::Arc;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let api_worker_addr = Address::random_local();
    let auth_worker_addr = Address::random_local();

    // create 2 identities to populate the trusted identities
    let identities = identities().await?;
    let auth_identifier = identities.identities_creation().create_identity().await?;
    let member_identifier = identities.identities_creation().create_identity().await?;
    let member_identity = identities.get_identity(&member_identifier).await?;

    let now = now().unwrap();

    let pre_trusted = BTreeMap::from([(
        member_identifier.clone(),
        PreTrustedIdentity::new(
            BTreeMap::from([(b"attr".to_vec(), b"value".to_vec())]),
            now,
            None,
            auth_identifier.clone(),
        ),
    )]);

    let members = Arc::new(AuthorityMembersSqlxDatabase::create().await?);
    members
        .bootstrap_pre_trusted_members(&auth_identifier, &pre_trusted.into())
        .await?;

    // Now recreate the identities services with the previous vault
    // (so that the authority can verify its signature)
    // and the repository containing the trusted identities
    let identities = Identities::builder()
        .await?
        .with_change_history_repository(identities.change_history_repository())
        .with_vault(identities.vault())
        .with_purpose_keys_repository(identities.purpose_keys_repository())
        .with_cached_credential_repository(identities.cached_credentials_repository())
        .build();
    let secure_channels = SecureChannels::from_identities(
        identities.clone(),
        Arc::new(SecureChannelSqlxDatabase::create().await?),
    );
    let identities_verification = identities.identities_verification();

    // Create the CredentialIssuer:
    let options = SecureChannelListenerOptions::new();
    let sc_flow_control_id = options.spawner_flow_control_id();
    secure_channels.create_secure_channel_listener(
        ctx,
        &auth_identifier,
        api_worker_addr.clone(),
        options,
    )?;
    ctx.flow_controls()
        .add_consumer(&auth_worker_addr, &sc_flow_control_id);
    let auth = CredentialIssuerWorker::new(
        members,
        identities.identities_attributes(),
        identities.credentials(),
        &auth_identifier,
        "test".to_string(),
        None,
        None,
        true,
    );
    ctx.start_worker(auth_worker_addr.clone(), auth)?;

    // Connect to the API channel from the member:
    let e2a = secure_channels
        .create_secure_channel(
            ctx,
            &member_identifier,
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

    let imported = identities_verification
        .import(Some(&member_identifier), &exported)
        .await
        .unwrap();
    let data = identities
        .credentials()
        .credentials_verification()
        .verify_credential(Some(&imported), &[auth_identifier.clone()], &credential)
        .await?;
    assert_eq!(
        Some(&b"value".to_vec().into()),
        data.credential_data
            .subject_attributes
            .map
            .get::<ByteSlice>(b"attr".as_slice().into())
    );
    Ok(())
}
