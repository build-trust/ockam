use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{async_trait, AllowAll, Any, DenyAll, Mailboxes};
use ockam_core::{route, Result, Routed, Worker};
use ockam_identity::authenticated_storage::{
    mem::InMemoryStorage, AuthenticatedAttributeStorage, IdentityAttributeStorage,
    IdentityAttributeStorageReader,
};
use ockam_identity::credential::access_control::CredentialAccessControl;
use ockam_identity::credential::Credential;
use ockam_identity::{
    AuthorityInfo, CredentialMemoryRetriever, Identity, SecureChannelListenerOptions,
    SecureChannelOptions, TrustContext, TrustIdentifierPolicy,
};

use ockam_node::{Context, MessageSendReceiveOptions, WorkerBuilder};
use ockam_vault::Vault;
use std::sync::atomic::{AtomicI8, Ordering};
use std::time::Duration;

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();

    let authenticated_attribute_storage =
        AuthenticatedAttributeStorage::new(Arc::new(InMemoryStorage::new()));

    let authority = Identity::create(ctx, vault.clone()).await?;
    let server = Identity::create(ctx, vault.clone()).await?;

    server
        .create_secure_channel_listener("listener", SecureChannelListenerOptions::new())
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityInfo::new(authority.to_public().await?, None)),
    );

    server
        .start_credential_exchange_worker(
            trust_context,
            "credential_exchange",
            false,
            Arc::new(authenticated_attribute_storage.clone()),
        )
        .await?;

    let client = Identity::create(ctx, vault).await?;
    let channel = client
        .create_secure_channel(
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.identifier().clone())),
        )
        .await?;

    let credential_builder = Credential::builder(client.identifier().clone());
    let credential = credential_builder.with_attribute("is_superuser", b"true");

    let credential = authority.issue_credential(credential).await?;

    client
        .present_credential(
            route![channel, "credential_exchange"],
            &credential,
            MessageSendReceiveOptions::new(),
        )
        .await?;

    let attrs = authenticated_attribute_storage
        .get_attributes(client.identifier())
        .await?
        .unwrap();

    let val = attrs.attrs().get("is_superuser").unwrap();

    assert_eq!(val.as_slice(), b"true");

    ctx.stop().await
}

#[ockam_macros::test]
async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
    let vault = Vault::create();
    let authenticated_attribute_storage_client_1 =
        AuthenticatedAttributeStorage::new(Arc::new(InMemoryStorage::new()));
    let storage2 = Arc::new(InMemoryStorage::new());
    let authenticated_attribute_storage_client_2 =
        AuthenticatedAttributeStorage::new(storage2.clone());

    let authority = Identity::create(ctx, vault.clone()).await?;
    let client2 = Identity::create(ctx, vault.clone()).await?;

    let credential2 =
        Credential::builder(client2.identifier().clone()).with_attribute("is_admin", b"true");

    let credential2 = authority.issue_credential(credential2).await?;

    client2
        .create_secure_channel_listener("listener", SecureChannelListenerOptions::new())
        .await?;
    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityInfo::new(
            authority.to_public().await?,
            Some(Arc::new(CredentialMemoryRetriever::new(credential2))),
        )),
    );
    let authorities = vec![trust_context.authority()?.identity().clone()];

    client2
        .start_credential_exchange_worker(
            trust_context,
            "credential_exchange",
            true,
            Arc::new(authenticated_attribute_storage_client_2),
        )
        .await?;

    let client1 = Identity::create(ctx, vault).await?;

    let credential1 =
        Credential::builder(client1.identifier().clone()).with_attribute("is_user", b"true");

    let credential1 = authority.issue_credential(credential1).await?;

    let channel = client1
        .create_secure_channel(route!["listener"], SecureChannelOptions::new())
        .await?;

    let storage: Arc<dyn IdentityAttributeStorage> =
        Arc::new(authenticated_attribute_storage_client_1.clone());
    client1
        .present_credential_mutual(
            route![channel, "credential_exchange"],
            &authorities,
            storage,
            &credential1,
            MessageSendReceiveOptions::new(),
        )
        .await?;

    let attrs1 = AuthenticatedAttributeStorage::new(storage2.clone())
        .get_attributes(client1.identifier())
        .await?
        .unwrap();

    assert_eq!(attrs1.attrs().get("is_user").unwrap().as_slice(), b"true");

    let attrs2 = authenticated_attribute_storage_client_1
        .get_attributes(client2.identifier())
        .await?
        .unwrap();

    assert_eq!(attrs2.attrs().get("is_admin").unwrap().as_slice(), b"true");

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
    let storage = Arc::new(InMemoryStorage::new());
    let authority = Identity::create(ctx, vault.clone()).await?;
    let server = Identity::create(ctx, vault.clone()).await?;

    server
        .create_secure_channel_listener("listener", SecureChannelListenerOptions::new())
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityInfo::new(authority.to_public().await?, None)),
    );

    server
        .start_credential_exchange_worker(
            trust_context,
            "credential_exchange",
            false,
            Arc::new(AuthenticatedAttributeStorage::new(storage.clone())),
        )
        .await?;

    let client = Identity::create(ctx, vault.clone()).await?;
    let channel = client
        .create_secure_channel(
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.identifier().clone())),
        )
        .await?;

    let credential_builder = Credential::builder(client.identifier().clone());
    let credential = credential_builder.with_attribute("is_superuser", b"true");

    let credential = authority.issue_credential(credential).await?;

    let counter = Arc::new(AtomicI8::new(0));

    let worker = CountingWorker {
        msgs_count: counter.clone(),
    };

    let required_attributes = vec![("is_superuser".to_string(), b"true".to_vec())];
    let access_control = CredentialAccessControl::new(
        &required_attributes,
        AuthenticatedAttributeStorage::new(storage.clone()),
    );

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
        .present_credential(
            route![channel.clone(), "credential_exchange"],
            &credential,
            MessageSendReceiveOptions::new(),
        )
        .await?;

    child_ctx
        .send(route![channel, "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 1);

    ctx.stop().await
}
