use std::sync::atomic::{AtomicI8, Ordering};
use std::time::Duration;

use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Any, DenyAll};
use ockam_core::{route, Result, Routed, Worker};
use ockam_identity::models::CredentialSchemaIdentifier;
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{
    AuthorityService, CredentialAccessControl, CredentialsMemoryRetriever,
    SecureChannelListenerOptions, SecureChannelOptions, TrustContext, TrustIdentifierPolicy,
};
use ockam_node::{Context, WorkerBuilder};

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identity_attributes_repository = identities.identity_attributes_repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            secure_channels.identities().credentials(),
            authority.clone(),
            None,
        )),
    );

    ctx.flow_controls()
        .add_consumer("credential_exchange", listener.flow_control_id());
    credentials_service
        .start(
            ctx,
            trust_context,
            server.clone(),
            "credential_exchange".into(),
            false,
        )
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.clone())),
        )
        .await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60),
        )
        .await?;

    credentials_service
        .present_credential(ctx, route![channel, "credential_exchange"], credential)
        .await?;

    let attrs = identity_attributes_repository
        .get_attributes(&client)
        .await?
        .unwrap();

    let val = attrs.get("is_superuser".into()).unwrap();

    assert_eq!(val, "true".into());

    ctx.stop().await
}

#[ockam_macros::test]
async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identity_attributes_repository = identities.identity_attributes_repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let client1 = identities_creation.create_identity().await?;
    let client2 = identities_creation.create_identity().await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client1,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_admin", "true")
                .build(),
            Duration::from_secs(60),
        )
        .await?;

    let listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &client1,
            "listener",
            SecureChannelListenerOptions::new(),
        )
        .await?;
    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            secure_channels.identities().credentials(),
            authority.clone(),
            Some(Arc::new(CredentialsMemoryRetriever::new(credential))),
        )),
    );
    ctx.flow_controls()
        .add_consumer("credential_exchange", listener.flow_control_id());

    credentials_service
        .start(
            ctx,
            trust_context.clone(),
            client1.clone(),
            "credential_exchange".into(),
            true,
        )
        .await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client2,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_user", "true")
                .build(),
            Duration::from_secs(60),
        )
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client2,
            route!["listener"],
            SecureChannelOptions::new(),
        )
        .await?;

    credentials_service
        .present_credential_mutual(
            ctx,
            route![channel, "credential_exchange"],
            &trust_context.authorities(),
            credential,
        )
        .await?;

    let attrs1 = identity_attributes_repository
        .get_attributes(&client1)
        .await?
        .unwrap();

    assert_eq!(attrs1.get("is_admin".into()).unwrap(), "true".into());

    let attrs2 = identity_attributes_repository
        .get_attributes(&client2)
        .await?
        .unwrap();

    assert_eq!(attrs2.get("is_user".into()).unwrap(), "true".into());

    ctx.stop().await
}

#[ockam_macros::test]
async fn access_control(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identity_attributes_repository = identities.identity_attributes_repository();
    let credentials = identities.credentials();
    let credentials_service = identities.credentials_server();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let options = SecureChannelListenerOptions::new();
    let listener = secure_channels
        .create_secure_channel_listener(ctx, &server, "listener", options)
        .await?;

    let trust_context = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(AuthorityService::new(
            credentials.clone(),
            authority.clone(),
            None,
        )),
    );

    ctx.flow_controls()
        .add_consumer("credential_exchange", listener.flow_control_id());

    credentials_service
        .start(
            ctx,
            trust_context,
            server.clone(),
            "credential_exchange".into(),
            false,
        )
        .await?;

    let channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.clone())),
        )
        .await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60),
        )
        .await?;

    let counter = Arc::new(AtomicI8::new(0));

    let worker = CountingWorker {
        msgs_count: counter.clone(),
    };

    let required_attributes = vec![("is_superuser".into(), "true".into())];
    let access_control =
        CredentialAccessControl::new(&required_attributes, identity_attributes_repository.clone());

    ctx.flow_controls()
        .add_consumer("counter", listener.flow_control_id());

    WorkerBuilder::new(worker)
        .with_address("counter")
        .with_incoming_access_control(access_control)
        .with_outgoing_access_control(DenyAll)
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
