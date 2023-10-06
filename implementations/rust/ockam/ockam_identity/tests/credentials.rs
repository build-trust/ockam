use std::sync::atomic::{AtomicI8, Ordering};
use std::time::Duration;

use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Any, DenyAll};
use ockam_core::{route, Result, Routed, Worker};
use ockam_identity::models::CredentialSchemaIdentifier;
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{
    CredentialAccessControl, SecureChannelListenerOptions, SecureChannelOptions,
    TrustIdentifierPolicy,
};
use ockam_node::{Context, WorkerBuilder};

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let credentials = identities.credentials();

    let authority = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();
    let server = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();
    let client = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "listener",
            SecureChannelListenerOptions::new().with_authority(authority.clone()),
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

    secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.clone()))
                .with_credential(credential)?,
        )
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    let attrs = identities_repository
        .get_attributes(&client)
        .await?
        .unwrap();

    let val = attrs.attrs().get("is_superuser".as_bytes()).unwrap();

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

    let authority = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();

    let client1 = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();
    let client2 = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();

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

    secure_channels
        .create_secure_channel_listener(
            ctx,
            &client1,
            "listener",
            SecureChannelListenerOptions::new()
                .with_credential(credential)?
                .with_authority(authority.clone()),
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

    secure_channels
        .create_secure_channel(
            ctx,
            &client2,
            route!["listener"],
            SecureChannelOptions::new()
                .with_credential(credential)?
                .with_authority(authority.clone()),
        )
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    let attrs1 = identities_repository
        .get_attributes(&client1)
        .await?
        .unwrap();

    assert_eq!(
        attrs1
            .attrs()
            .get("is_admin".as_bytes())
            .unwrap()
            .as_slice(),
        b"true"
    );

    let attrs2 = identities_repository
        .get_attributes(&client2)
        .await?
        .unwrap();

    assert_eq!(
        attrs2.attrs().get("is_user".as_bytes()).unwrap().as_slice(),
        b"true"
    );

    ctx.stop().await
}

#[ockam_macros::test]
async fn access_control(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_repository = identities.repository();
    let credentials = identities.credentials();

    let authority = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();

    let server = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();
    let client1 = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();
    let client2 = identities_creation
        .create_identity()
        .await?
        .identifier()
        .clone();

    let options = SecureChannelListenerOptions::new().with_authority(authority.clone());
    let listener = secure_channels
        .create_secure_channel_listener(ctx, &server, "listener", options)
        .await?;

    let credential1 = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client1,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60),
        )
        .await?;
    let channel1 = secure_channels
        .create_secure_channel(
            ctx,
            &client1,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.clone()))
                .with_credential(credential1)?,
        )
        .await?;
    let channel2 = secure_channels
        .create_secure_channel(
            ctx,
            &client2,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_policy(TrustIdentifierPolicy::new(server.clone())),
        )
        .await?;

    let counter = Arc::new(AtomicI8::new(0));

    let worker = CountingWorker {
        msgs_count: counter.clone(),
    };

    let required_attributes = vec![(b"is_superuser".to_vec(), b"true".to_vec())];
    let access_control =
        CredentialAccessControl::new(&required_attributes, identities_repository.clone());

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

    ctx.send(route![channel2.clone(), "counter"], "Hello".to_string())
        .await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::Relaxed), 0);

    ctx.send(route![channel1, "counter"], "Hello".to_string())
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
