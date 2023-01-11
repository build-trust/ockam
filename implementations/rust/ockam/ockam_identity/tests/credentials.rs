use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{async_trait, AllowAll, Any, DenyAll, Mailboxes};
use ockam_core::{route, Result, Routed, Worker};
use ockam_identity::authenticated_storage::mem::InMemoryStorage;
use ockam_identity::credential::access_control::CredentialAccessControl;
use ockam_identity::credential::{AttributesStorageUtils, Credential};
use ockam_identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy, TrustIdentifierPolicy};
use ockam_node::{Context, WorkerBuilder};
use ockam_vault::Vault;
use std::sync::atomic::{AtomicI8, Ordering};
use std::time::Duration;

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let authority = Identity::create(ctx, &vault).await?;

    let server = Identity::create(ctx, &vault).await?;
    let server_storage = InMemoryStorage::new();
    let registry = SecureChannelRegistry::new();

    server
        .create_secure_channel_listener("listener", TrustEveryonePolicy, &server_storage, &registry)
        .await?;

    let authorities = vec![authority.to_public().await?];

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
            &registry,
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

    let registry = SecureChannelRegistry::new();

    let authority = Identity::create(ctx, &vault).await?;

    let client2 = Identity::create(ctx, &vault).await?;
    let client2_storage = InMemoryStorage::new();

    let credential2 =
        Credential::builder(client2.identifier().clone()).with_attribute("is_admin", b"true");

    let credential2 = authority.issue_credential(credential2).await?;
    client2.set_credential(Some(credential2)).await;

    client2
        .create_secure_channel_listener(
            "listener",
            TrustEveryonePolicy,
            &client2_storage,
            &registry,
        )
        .await?;

    let authorities = vec![authority.to_public().await?];
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
        .create_secure_channel(
            route!["listener"],
            TrustEveryonePolicy,
            &client1_storage,
            &registry,
        )
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

struct CountingWorker {
    msgs_count: Arc<AtomicI8>,
}

#[async_trait]
impl Worker for CountingWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        let _ = self.msgs_count.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }
}

#[ockam_macros::test]
async fn access_control(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let registry = SecureChannelRegistry::new();

    let authority = Identity::create(ctx, &vault).await?;

    let server = Identity::create(ctx, &vault).await?;
    let server_storage = InMemoryStorage::new();

    server
        .create_secure_channel_listener("listener", TrustEveryonePolicy, &server_storage, &registry)
        .await?;

    let authorities = vec![authority.to_public().await?];

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
            &registry,
        )
        .await?;

    let credential_builder = Credential::builder(client.identifier().clone());
    let credential = credential_builder.with_attribute("is_superuser", b"true");

    let credential = authority.issue_credential(credential).await?;

    client.set_credential(Some(credential)).await;

    let counter = Arc::new(AtomicI8::new(0));

    let worker = CountingWorker {
        msgs_count: counter.clone(),
    };

    let required_attributes = vec![("is_superuser".to_string(), b"true".to_vec())];
    let access_control = CredentialAccessControl::new(&required_attributes, server_storage);

    WorkerBuilder::with_access_control(
        Arc::new(access_control),
        Arc::new(DenyAll),
        "counter",
        worker,
    )
    .start(ctx)
    .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    let child_ctx = ctx
        .new_detached_with_mailboxes(Mailboxes::main(
            "child",
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        ))
        .await?;

    child_ctx
        .send(route![channel.clone(), "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    client
        .present_credential(route![channel.clone(), "credential_exchange"])
        .await?;

    child_ctx
        .send(route![channel, "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    ctx.stop().await
}
