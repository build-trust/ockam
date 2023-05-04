use std::sync::atomic::{AtomicI8, Ordering};
use std::time::Duration;

use ockam_core::compat::sync::Arc;
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{async_trait, Any, DenyAll};
use ockam_core::{route, Result, Routed, Worker};
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::{
    AuthorityService, CredentialAccessControl, CredentialData, CredentialsMemoryRetriever,
    SecureChannelListenerOptions, SecureChannelOptions, TrustContext, TrustIdentifierPolicy,
};
use ockam_node::{Context, WorkerBuilder};

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &server.identifier(),
            "listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            secure_channels.identities().identities_reader(),
            secure_channels.identities().credentials(),
            authority.identifier(),
            None,
        )),
    );

    ctx.flow_controls().add_consumer(
        "credential_exchange",
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );
    credentials_service
        .start(
            ctx,
            trust_context,
            server.identifier(),
            "credential_exchange".into(),
            false,
        )
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client.identifier(),
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.identifier().clone())),
        )
        .await?;

    let credential_data = CredentialData::builder(client.identifier(), authority.identifier())
        .with_attribute("is_superuser", b"true")
        .build()?;

    let credential = credentials
        .issue_credential(&authority.identifier(), credential_data)
        .await?;

    credentials_service
        .present_credential(ctx, route![channel, "credential_exchange"], credential)
        .await?;

    let attrs = identities_repository
        .get_attributes(&client.identifier())
        .await?
        .unwrap();

    let val = attrs.attrs().get("is_superuser").unwrap();

    assert_eq!(val.as_slice(), b"true");

    ctx.stop().await
}

#[ockam_macros::test]
async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let client1 = identities_creation.create_identity().await?;
    let client2 = identities_creation.create_identity().await?;

    let credential_data = CredentialData::builder(client1.identifier(), authority.identifier())
        .with_attribute("is_admin", b"true")
        .build()?;
    let credential = credentials
        .issue_credential(&authority.identifier(), credential_data)
        .await?;

    let listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &client1.identifier(),
            "listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;
    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            secure_channels.identities().identities_reader(),
            secure_channels.identities().credentials(),
            authority.identifier(),
            Some(Arc::new(CredentialsMemoryRetriever::new(credential))),
        )),
    );
    ctx.flow_controls().add_consumer(
        "credential_exchange",
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    credentials_service
        .start(
            ctx,
            trust_context.clone(),
            client1.identifier(),
            "credential_exchange".into(),
            true,
        )
        .await?;

    let credential_data = CredentialData::builder(client2.identifier(), authority.identifier())
        .with_attribute("is_user", b"true")
        .build()?;
    let credential = credentials
        .issue_credential(&authority.identifier(), credential_data)
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client2.identifier(),
            route!["listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    credentials_service
        .present_credential_mutual(
            ctx,
            route![channel, "credential_exchange"],
            trust_context.authorities().await?.as_slice(),
            credential,
        )
        .await?;

    let attrs1 = identities_repository
        .get_attributes(&client1.identifier())
        .await?
        .unwrap();

    assert_eq!(attrs1.attrs().get("is_admin").unwrap().as_slice(), b"true");

    let attrs2 = identities_repository
        .get_attributes(&client2.identifier())
        .await?
        .unwrap();

    assert_eq!(attrs2.attrs().get("is_user").unwrap().as_slice(), b"true");

    ctx.stop().await
}

#[ockam_macros::test]
async fn access_control(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let options = SecureChannelListenerOptions::new();
    let listener = secure_channels
        .create_secure_channel_listener(ctx, &server.identifier(), "listener", options)
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            identities.identities_reader(),
            credentials.clone(),
            authority.identifier(),
            None,
        )),
    );

    ctx.flow_controls().add_consumer(
        "credential_exchange",
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    credentials_service
        .start(
            ctx,
            trust_context,
            server.identifier(),
            "credential_exchange".into(),
            false,
        )
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client.identifier(),
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.identifier().clone())),
        )
        .await?;

    let credential_data = CredentialData::builder(client.identifier(), authority.identifier())
        .with_attribute("is_superuser", b"true")
        .build()?;
    let credential = credentials
        .issue_credential(&authority.identifier(), credential_data)
        .await?;

    let counter = Arc::new(AtomicI8::new(0));

    let worker = CountingWorker {
        msgs_count: counter.clone(),
    };

    let required_attributes = vec![("is_superuser".to_string(), b"true".to_vec())];
    let access_control =
        CredentialAccessControl::new(&required_attributes, identities_repository.clone());

    ctx.flow_controls().add_consumer(
        "counter",
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
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

    ctx.send(route![channel.clone(), "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    credentials_service
        .present_credential(
            ctx,
            route![channel.clone(), "credential_exchange"],
            credential,
        )
        .await?;

    ctx.send(route![channel, "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 1);

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
