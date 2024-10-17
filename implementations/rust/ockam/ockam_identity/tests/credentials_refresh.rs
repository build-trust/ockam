use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use ockam_core::api::Response;
use ockam_core::compat::sync::Arc;
use ockam_core::{async_trait, Any, AsyncTryClone, Routed, SecureChannelLocalInfo, Worker};
use ockam_core::{route, Result};
use ockam_identity::models::CredentialSchemaIdentifier;
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{
    Credentials, Identifier, RemoteCredentialRetrieverCreator, RemoteCredentialRetrieverInfo,
    RemoteCredentialRetrieverTimingOptions, SecureChannelListenerOptions, SecureChannelOptions,
    SecureChannels,
};
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;

struct CredentialIssuer {
    delay: Duration,
    call_counter: Arc<AtomicU64>,
    pause: Arc<AtomicBool>,
    credentials: Arc<Credentials>,
    authority: Identifier,
    ttl: Duration,
}

#[async_trait]
impl Worker for CredentialIssuer {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        if self.pause.load(Ordering::Relaxed) {
            return Ok(());
        }

        let subject = SecureChannelLocalInfo::find_info(msg.local_message())?
            .their_identifier()
            .into();
        let credential = self
            .credentials
            .credentials_creation()
            .issue_credential(
                &self.authority,
                &subject,
                AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
                    .with_attribute(b"key", b"value")
                    .build(),
                self.ttl,
            )
            .await?;

        let response = Response::ok().body(credential).to_vec()?;

        self.call_counter.fetch_add(1, Ordering::Relaxed);

        ctx.sleep(self.delay).await;
        ctx.send(msg.return_route(), response).await?;

        Ok(())
    }
}

#[ockam_macros::test]
async fn autorefresh(ctx: &mut Context) -> Result<()> {
    let timing_options = RemoteCredentialRetrieverTimingOptions {
        min_refresh_interval: Duration::from_secs(1),
        proactive_refresh_gap: 1.into(),
        clock_skew_gap: 0.into(),
        request_timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let res = init(
        ctx,
        Duration::from_secs(0),
        Duration::from_secs(5),
        timing_options,
    )
    .await?;

    assert_eq!(res.call_counter.load(Ordering::Relaxed), 0);
    let _channel = res
        .client_secure_channels
        .create_secure_channel(
            ctx,
            &res.client,
            route!["server_api"],
            SecureChannelOptions::new()
                .with_credential_retriever_creator(res.retriever)?
                .with_authority(res.authority.clone()),
        )
        .await?;

    assert_eq!(res.call_counter.load(Ordering::Relaxed), 1);
    ctx.sleep(Duration::from_secs(1)).await;

    let server_attrs1 = res
        .server_secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&res.client, &res.authority)
        .await?
        .unwrap();

    ctx.sleep(Duration::from_secs(4)).await;
    assert_eq!(res.call_counter.load(Ordering::Relaxed), 2);

    ctx.sleep(Duration::from_secs(9)).await;
    assert_eq!(res.call_counter.load(Ordering::Relaxed), 4);

    let server_attrs2 = res
        .server_secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&res.client, &res.authority)
        .await?
        .unwrap();

    assert!(server_attrs2.added_at() > server_attrs1.added_at());

    // Shut down Authority and check that credentials expired
    res.pause.store(true, Ordering::Relaxed);

    ctx.sleep(Duration::from_secs(6)).await;
    assert!(res
        .server_secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&res.client, &res.authority)
        .await?
        .is_none());

    // Enable Authority node again and check that everything is back to normal
    res.pause.store(false, Ordering::Relaxed);
    ctx.sleep(Duration::from_secs(4)).await;

    let server_attrs3 = res
        .server_secure_channels
        .identities()
        .identities_attributes()
        .get_attributes(&res.client, &res.authority)
        .await?
        .unwrap();
    assert!(server_attrs3.added_at() > server_attrs2.added_at());

    Ok(())
}

#[ockam_macros::test]
async fn init_fail(ctx: &mut Context) -> Result<()> {
    let timing_options = RemoteCredentialRetrieverTimingOptions {
        min_refresh_interval: Duration::from_secs(1),
        proactive_refresh_gap: 1.into(),
        clock_skew_gap: 0.into(),
        request_timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let res = init(
        ctx,
        Duration::from_secs(0),
        Duration::from_secs(5),
        timing_options,
    )
    .await?;

    res.pause.store(true, Ordering::Relaxed);
    // If Authority is inaccessible secure creation should fail
    let channel = res
        .client_secure_channels
        .create_secure_channel(
            ctx,
            &res.client,
            route!["server_api"],
            SecureChannelOptions::new()
                .with_credential_retriever_creator(res.retriever)?
                .with_authority(res.authority.clone()),
        )
        .await;

    assert!(channel.is_err());

    Ok(())
}

#[allow(dead_code)]
struct InitResult {
    call_counter: Arc<AtomicU64>,
    pause: Arc<AtomicBool>,

    client: Identifier,
    server: Identifier,
    authority: Identifier,

    client_secure_channels: Arc<SecureChannels>,
    server_secure_channels: Arc<SecureChannels>,
    authority_secure_channels: Arc<SecureChannels>,

    retriever: Arc<RemoteCredentialRetrieverCreator>,
}

async fn init(
    ctx: &Context,
    delay: Duration,
    ttl: Duration,
    timing_options: RemoteCredentialRetrieverTimingOptions,
) -> Result<InitResult> {
    let tcp = TcpTransport::create(ctx).await?;

    let client_secure_channels = secure_channels().await?;
    let authority_secure_channels = secure_channels().await?;
    let server_secure_channels = secure_channels().await?;

    let client_identities = client_secure_channels.identities();
    let authority_identities = authority_secure_channels.identities();
    let server_identities = server_secure_channels.identities();

    let client_identities_creation = client_identities.identities_creation();
    let authority_identities_creation = authority_identities.identities_creation();
    let server_identities_creation = server_identities.identities_creation();

    let client = client_identities_creation.create_identity().await?;
    let authority = authority_identities_creation.create_identity().await?;
    let server = server_identities_creation.create_identity().await?;

    let authority_identity = authority_secure_channels
        .identities()
        .export_identity(&authority)
        .await?;
    server_identities
        .identities_verification()
        .import(Some(&authority), &authority_identity)
        .await?;
    client_identities
        .identities_verification()
        .import(Some(&authority), &authority_identity)
        .await?;

    let call_counter = Arc::new(AtomicU64::new(0));
    let pause = Arc::new(AtomicBool::new(false));
    let issuer = CredentialIssuer {
        delay,
        call_counter: call_counter.clone(),
        pause: pause.clone(),
        credentials: authority_identities.credentials(),
        authority: authority.clone(),
        ttl,
    };

    ctx.start_worker("credential_issuer", issuer).await?;

    let listener = authority_secure_channels
        .create_secure_channel_listener(
            ctx,
            &authority,
            "authority_api",
            SecureChannelListenerOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer("credential_issuer", listener.flow_control_id());

    server_secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "server_api",
            SecureChannelListenerOptions::new().with_authority(authority.clone()),
        )
        .await?;

    let retriever = Arc::new(RemoteCredentialRetrieverCreator::new_extended(
        ctx.async_try_clone().await?,
        Arc::new(tcp),
        client_secure_channels.clone(),
        RemoteCredentialRetrieverInfo::create_for_project_member(
            authority.clone(),
            route!["authority_api"],
        ),
        "test".to_string(),
        timing_options,
    ));

    Ok(InitResult {
        call_counter,
        pause,
        client,
        server,
        authority,
        client_secure_channels,
        server_secure_channels,
        authority_secure_channels,
        retriever,
    })
}
