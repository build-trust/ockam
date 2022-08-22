use ockam_core::{route, Result};
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::credential::{AttributesStorageUtils, Credential};
use ockam_identity::{Identity, TrustEveryonePolicy, TrustIdentifierPolicy};
use ockam_node::Context;
use ockam_vault::Vault;
use std::collections::BTreeMap;

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let authority = Identity::create(ctx, &vault).await?;

    let server = Identity::create(ctx, &vault).await?;
    let server_storage = InMemoryStorage::new();

    server
        .create_secure_channel_listener("listener", TrustEveryonePolicy, &server_storage)
        .await?;

    let mut authorities = BTreeMap::new();
    authorities.insert(authority.identifier().clone(), authority.to_public().await?);
    server
        .start_credentials_exchange_worker(
            authorities,
            "credential_exchange",
            false,
            server_storage.clone(),
        )
        .await?;

    let client = Identity::create(ctx, &vault).await?;
    let client_storage = InMemoryStorage::new();
    let channel = client
        .create_secure_channel(
            route!["listener"],
            TrustIdentifierPolicy::new(server.identifier().clone()),
            &client_storage,
        )
        .await?;

    let credential_builder = Credential::builder(client.identifier().clone());
    let credential = credential_builder.with_attribute("is_superuser", b"true");

    let credential = authority.issue_credential(credential).await?;

    client.set_credential(Some(credential)).await;

    client
        .present_credential(route![channel, "credential_exchange"])
        .await?;

    let attrs = AttributesStorageUtils::get_attributes(client.identifier(), &server_storage)
        .await?
        .unwrap();

    let val = attrs.get("is_superuser").unwrap();

    assert_eq!(val.as_slice(), b"true");

    ctx.stop().await
}

#[ockam_macros::test]
async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let authority = Identity::create(ctx, &vault).await?;

    let client2 = Identity::create(ctx, &vault).await?;
    let client2_storage = InMemoryStorage::new();

    let credential2 =
        Credential::builder(client2.identifier().clone()).with_attribute("is_admin", b"true");

    let credential2 = authority.issue_credential(credential2).await?;
    client2.set_credential(Some(credential2)).await;

    client2
        .create_secure_channel_listener("listener", TrustEveryonePolicy, &client2_storage)
        .await?;

    let mut authorities = BTreeMap::new();
    authorities.insert(authority.identifier().clone(), authority.to_public().await?);
    client2
        .start_credentials_exchange_worker(
            authorities.clone(),
            "credential_exchange",
            true,
            client2_storage.clone(),
        )
        .await?;

    let client1 = Identity::create(ctx, &vault).await?;
    let client1_storage = InMemoryStorage::new();

    let credential1 =
        Credential::builder(client1.identifier().clone()).with_attribute("is_user", b"true");

    let credential1 = authority.issue_credential(credential1).await?;
    client1.set_credential(Some(credential1)).await;

    let channel = client1
        .create_secure_channel(route!["listener"], TrustEveryonePolicy, &client1_storage)
        .await?;

    client1
        .present_credential_mutual(
            route![channel, "credential_exchange"],
            &authorities,
            &client1_storage,
        )
        .await?;

    let attrs1 = AttributesStorageUtils::get_attributes(client1.identifier(), &client2_storage)
        .await?
        .unwrap();

    assert_eq!(attrs1.get("is_user").unwrap().as_slice(), b"true");

    let attrs2 = AttributesStorageUtils::get_attributes(client2.identifier(), &client1_storage)
        .await?
        .unwrap();

    assert_eq!(attrs2.get("is_admin").unwrap().as_slice(), b"true");

    ctx.stop().await
}
