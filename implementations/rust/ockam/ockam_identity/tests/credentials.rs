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
use ockam_node::workers::Echoer;
use ockam_node::{Context, WorkerBuilder};

#[ockam_macros::test]
async fn full_flow_oneway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_attributes = identities.identities_attributes();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    secure_channels.create_secure_channel_listener(
        ctx,
        &server,
        "listener",
        SecureChannelListenerOptions::new().with_authority(authority.clone()),
    )?;

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

    ctx.sleep(Duration::from_millis(200)).await;

    let attrs = identities_attributes
        .get_attributes(&client, &authority)
        .await?
        .unwrap();

    let val = attrs.attrs().get("is_superuser".as_bytes()).unwrap();

    assert_eq!(val.as_slice(), b"true");

    Ok(())
}

#[ockam_macros::test]
async fn full_flow_twoway(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_attributes = identities.identities_attributes();
    let credentials = identities.credentials();

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
            Duration::from_secs(60 * 60),
        )
        .await?;

    secure_channels.create_secure_channel_listener(
        ctx,
        &client1,
        "listener",
        SecureChannelListenerOptions::new()
            .with_authority(authority.clone())
            .with_credential(credential)?,
    )?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client2,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_user", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    secure_channels
        .create_secure_channel(
            ctx,
            &client2,
            route!["listener"],
            SecureChannelOptions::new()
                .with_authority(authority.clone())
                .with_credential(credential)?,
        )
        .await?;

    ctx.sleep(Duration::from_millis(200)).await;

    let attrs1 = identities_attributes
        .get_attributes(&client1, &authority)
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

    let attrs2 = identities_attributes
        .get_attributes(&client2, &authority)
        .await?
        .unwrap();

    assert_eq!(
        attrs2.attrs().get("is_user".as_bytes()).unwrap().as_slice(),
        b"true"
    );

    Ok(())
}

#[ockam_macros::test]
async fn access_control(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_attributes = identities.identities_attributes();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;

    let server = identities_creation.create_identity().await?;
    let client1 = identities_creation.create_identity().await?;
    let client2 = identities_creation.create_identity().await?;

    let options = SecureChannelListenerOptions::new().with_authority(authority.clone());
    let listener =
        secure_channels.create_secure_channel_listener(ctx, &server, "listener", options)?;

    let credential1 = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client1,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60 * 60),
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
        CredentialAccessControl::new(&required_attributes, authority, identities_attributes);

    ctx.flow_controls()
        .add_consumer(&"counter".into(), listener.flow_control_id());

    WorkerBuilder::new(worker)
        .with_address("counter")
        .with_incoming_access_control(access_control)
        .with_outgoing_access_control(DenyAll)
        .start(ctx)?;
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

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn missing_authority__handshake_should_succeed(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_attributes = identities.identities_attributes();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority,
            &client,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    let listener = secure_channels.create_secure_channel_listener(
        ctx,
        &server,
        "listener",
        SecureChannelListenerOptions::new(),
    )?;

    let sc = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new().with_credential(credential)?,
        )
        .await?;

    ctx.sleep(Duration::from_millis(200)).await;

    // No attributes because we didn't provide authority to the responder
    let attrs = identities_attributes
        .get_attributes(&client, &authority)
        .await?;

    assert!(attrs.is_none());

    // However secure channel should be operational anyways

    ctx.start_worker("echo", Echoer)?;
    ctx.flow_controls()
        .add_consumer(&"echo".into(), listener.flow_control_id());

    let msg: String = ctx
        .send_and_receive(route![sc, "echo"], "Test".to_string())
        .await?;
    assert_eq!(msg, "Test".to_string());

    Ok(())
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn invalid_credential__handshake_should_succeed(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let identities_attributes = identities.identities_attributes();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;
    let authority_wrong = identities_creation.create_identity().await?;
    let server = identities_creation.create_identity().await?;
    let client = identities_creation.create_identity().await?;

    let credential = credentials
        .credentials_creation()
        .issue_credential(
            &authority_wrong,
            &client,
            AttributesBuilder::with_schema(CredentialSchemaIdentifier(0))
                .with_attribute("is_superuser", "true")
                .build(),
            Duration::from_secs(60 * 60),
        )
        .await?;

    let listener = secure_channels.create_secure_channel_listener(
        ctx,
        &server,
        "listener",
        SecureChannelListenerOptions::new().with_authority(authority.clone()),
    )?;

    let sc = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new().with_credential(credential)?,
        )
        .await?;

    ctx.sleep(Duration::from_millis(200)).await;

    // No attributes because we provided wrong signature
    let attrs = identities_attributes
        .get_attributes(&client, &authority)
        .await?;

    assert!(attrs.is_none());

    let attrs = identities_attributes
        .get_attributes(&client, &authority_wrong)
        .await?;

    assert!(attrs.is_none());

    // However secure channel should be operational anyways

    ctx.start_worker("echo", Echoer)?;
    ctx.flow_controls()
        .add_consumer(&"echo".into(), listener.flow_control_id());

    let msg: String = ctx
        .send_and_receive(route![sc, "echo"], "Test".to_string())
        .await?;
    assert_eq!(msg, "Test".to_string());

    Ok(())
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
