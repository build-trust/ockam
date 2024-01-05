use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::{route, Result};
use ockam_identity::models::{CredentialAndPurposeKey, CredentialSchemaIdentifier};
use ockam_identity::secure_channels::secure_channels;
use ockam_identity::utils::AttributesBuilder;
use ockam_identity::{
    AuthorityService, Credentials, CredentialsRetriever, Identifier, IdentityError,
    SecureChannelListenerOptions, SecureChannelOptions, TrustContext,
};
use ockam_node::Context;

#[ockam_macros::test]
async fn autorefresh(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;

    let server = identities_creation.create_identity().await?;
    let call_counter_server = Arc::new(AtomicU8::new(0));
    let retriever_server = LocalCredentialsRetriever::new(
        credentials.clone(),
        authority.clone(),
        server.clone(),
        None,
        Duration::from_secs(5),
        Some(call_counter_server.clone()),
        None,
    );
    let authority_service_server = AuthorityService::new(
        credentials.clone(),
        authority.clone(),
        Some(Arc::new(retriever_server)),
    );
    let trust_context_server = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service_server),
    );
    let _listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "listener",
            SecureChannelListenerOptions::new()
                .with_trust_context(trust_context_server)
                .with_refresh_credential_time_gap(Duration::from_secs(1)),
        )
        .await?;

    let client = identities_creation.create_identity().await?;
    let call_counter_client = Arc::new(AtomicU8::new(0));
    let retriever_client = LocalCredentialsRetriever::new(
        credentials.clone(),
        authority.clone(),
        client.clone(),
        None,
        Duration::from_secs(4),
        Some(call_counter_client.clone()),
        None,
    );
    let authority_service_client = AuthorityService::new(
        credentials.clone(),
        authority.clone(),
        Some(Arc::new(retriever_client)),
    );
    let trust_context_client = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service_client),
    );
    let _channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_context(trust_context_client)
                .with_credential_refresh_time_gap(Duration::from_secs(1)),
        )
        .await?;

    ctx.sleep(Duration::from_secs(10)).await;

    // Client asks for a credential on second 0; 3; 6 and 9
    assert_eq!(call_counter_client.load(Ordering::Relaxed), 4);
    // Server asks for a credential on second 0; 4 and 8
    assert_eq!(call_counter_server.load(Ordering::Relaxed), 3);

    ctx.stop().await
}

#[ockam_macros::test]
async fn autorefresh_attributes_update(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;

    let server = identities_creation.create_identity().await?;
    let authority_service_server =
        AuthorityService::new(credentials.clone(), authority.clone(), None);
    let trust_context_server = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service_server),
    );
    let _listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &server,
            "listener",
            SecureChannelListenerOptions::new().with_trust_context(trust_context_server),
        )
        .await?;

    let client = identities_creation.create_identity().await?;
    let call_counter_client = Arc::new(AtomicU8::new(0));
    let retriever_client = LocalCredentialsRetriever::new(
        credentials.clone(),
        authority.clone(),
        client.clone(),
        None,
        Duration::from_secs(3),
        Some(call_counter_client.clone()),
        None,
    );
    let authority_service_client = AuthorityService::new(
        credentials.clone(),
        authority.clone(),
        Some(Arc::new(retriever_client)),
    );
    let trust_context_client = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service_client),
    );
    let _channel = secure_channels
        .create_secure_channel(
            ctx,
            &client,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_context(trust_context_client)
                .with_credential_refresh_time_gap(Duration::from_secs(1)),
        )
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;

    let attributes_reader = identities.identity_attributes_repository();

    let added1 = attributes_reader
        .get_attributes(&client)
        .await?
        .unwrap()
        .added();

    ctx.sleep(Duration::from_millis(3_100)).await;
    let added2 = attributes_reader
        .get_attributes(&client)
        .await?
        .unwrap()
        .added();
    let added3 = attributes_reader
        .get_attributes(&client)
        .await?
        .unwrap()
        .added();

    ctx.sleep(Duration::from_millis(3_100)).await;
    let added4 = attributes_reader
        .get_attributes(&client)
        .await?
        .unwrap()
        .added();

    assert!(added1 < added2);
    assert_eq!(added3, added2);
    assert!(added3 < added4);

    ctx.stop().await
}

#[ockam_macros::test]
async fn autorefresh_retry(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;
    let identities = secure_channels.identities();
    let identities_creation = identities.identities_creation();
    let credentials = identities.credentials();

    let authority = identities_creation.create_identity().await?;
    let client1 = identities_creation.create_identity().await?;
    let client2 = identities_creation.create_identity().await?;

    let authority_service2 = AuthorityService::new(credentials.clone(), authority.clone(), None);
    let trust_context2 = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service2),
    );
    let _listener = secure_channels
        .create_secure_channel_listener(
            ctx,
            &client2,
            "listener",
            SecureChannelListenerOptions::new().with_trust_context(trust_context2),
        )
        .await?;

    let call_counter = Arc::new(AtomicU8::new(0));
    let failed_call_counter = Arc::new(AtomicU8::new(0));
    let retriever = LocalCredentialsRetriever::new(
        credentials.clone(),
        authority.clone(),
        client1.clone(),
        Some(2), // Will fail on the second call
        Duration::from_secs(10),
        Some(call_counter.clone()),
        Some(failed_call_counter.clone()),
    );
    let authority_service1 = AuthorityService::new(
        credentials.clone(),
        authority.clone(),
        Some(Arc::new(retriever)),
    );
    let trust_context1 = TrustContext::new(
        "test_trust_context_id".to_string(),
        Some(authority_service1),
    );
    let _channel = secure_channels
        .create_secure_channel(
            ctx,
            &client1,
            route!["listener"],
            SecureChannelOptions::new()
                .with_trust_context(trust_context1)
                .with_credential_refresh_time_gap(Duration::from_secs(5))
                .with_min_credential_refresh_interval(Duration::from_secs(2)),
        )
        .await?;

    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(call_counter.load(Ordering::Relaxed), 1);
    assert_eq!(failed_call_counter.load(Ordering::Relaxed), 0);

    ctx.sleep(Duration::from_millis(5_100)).await;
    assert_eq!(call_counter.load(Ordering::Relaxed), 2);
    assert_eq!(failed_call_counter.load(Ordering::Relaxed), 1);

    ctx.sleep(Duration::from_millis(2_100)).await;
    assert_eq!(call_counter.load(Ordering::Relaxed), 3);
    assert_eq!(failed_call_counter.load(Ordering::Relaxed), 1);

    ctx.stop().await
}
struct LocalCredentialsRetriever {
    credentials: Arc<Credentials>,
    authority: Identifier,
    client: Identifier,
    fail_iteration: u8,
    ttl: Duration,
    call_counter: Arc<AtomicU8>,
    failed_call_counter: Arc<AtomicU8>,
}

impl LocalCredentialsRetriever {
    pub fn new(
        credentials: Arc<Credentials>,
        authority: Identifier,
        client: Identifier,
        fail_iteration: Option<u8>,
        ttl: Duration,
        call_counter: Option<Arc<AtomicU8>>,
        failed_call_counter: Option<Arc<AtomicU8>>,
    ) -> Self {
        Self {
            credentials,
            authority,
            client,
            fail_iteration: fail_iteration.unwrap_or(0),
            ttl,
            call_counter: call_counter.unwrap_or_default(),
            failed_call_counter: failed_call_counter.unwrap_or_default(),
        }
    }
}

#[async_trait]
impl CredentialsRetriever for LocalCredentialsRetriever {
    async fn retrieve(
        &self,
        _ctx: &Context,
        _for_identity: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        self.call_counter.fetch_add(1, Ordering::Relaxed);
        if self.fail_iteration == self.call_counter.load(Ordering::Relaxed) {
            self.failed_call_counter.fetch_add(1, Ordering::Relaxed);
            return Err(IdentityError::InvalidKeyData)?;
        }

        let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
            .with_attribute(b"name".to_vec(), b"client1".to_vec())
            .build();
        let credential = self
            .credentials
            .credentials_creation()
            .issue_credential(&self.authority, &self.client, attributes, self.ttl)
            .await?;
        Ok(credential)
    }
}
