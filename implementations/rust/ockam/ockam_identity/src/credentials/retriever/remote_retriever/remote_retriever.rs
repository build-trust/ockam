use core::cmp::max;
use tracing::{debug, error, info, trace, warn};

use ockam_core::api::Request;
use ockam_core::compat::string::String;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::{route, Address, Result};
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;
use ockam_transport_core::Transport;

use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{
    get_default_timeout, CachedCredentialRetriever, Identifier, RemoteCredentialRetrieverInfo,
    SecureChannels, SecureClient, TimestampInSeconds, DEFAULT_CREDENTIAL_CLOCK_SKEW_GAP,
};

/// This is the default interval before a credential expiration when we'll query for
/// a new credential to avoid it expiring before we got a new one.
pub const DEFAULT_PROACTIVE_REFRESH_CREDENTIAL_TIME_GAP: TimestampInSeconds =
    TimestampInSeconds(60);

/// Default minimal interval before 2 refreshed in case we retry the refresh.
pub const DEFAULT_MIN_REFRESH_CREDENTIAL_INTERVAL: Duration = Duration::from_secs(10);

/// Default timeout for requesting credential from the authority
pub const DEFAULT_CREDENTIAL_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Start refresh in the background before it expires
pub const DEFAULT_CREDENTIAL_PROACTIVE_REFRESH_GAP: TimestampInSeconds = TimestampInSeconds(60);

/// Timing options for retrieving remote credentials
#[derive(Clone, Copy)]
pub struct RemoteCredentialRetrieverTimingOptions {
    /// Timeout for request to the Authority node
    pub request_timeout: Duration,
    /// Timeout for creating secure channel to the Authority node
    pub secure_channel_creation_timeout: Duration,
    /// Minimum interval before refresh requests to the Authority node
    pub min_refresh_interval: Duration,
    /// Time gap used to request a new credential before the old one actually expires
    pub proactive_refresh_gap: TimestampInSeconds,
    /// Time gap used to consider credential expired before its actual expiration
    /// to account for time errors on different machines
    pub clock_skew_gap: TimestampInSeconds,
}

impl Default for RemoteCredentialRetrieverTimingOptions {
    fn default() -> Self {
        Self {
            request_timeout: get_default_timeout(),
            secure_channel_creation_timeout: get_default_timeout(),
            min_refresh_interval: DEFAULT_MIN_REFRESH_CREDENTIAL_INTERVAL,
            proactive_refresh_gap: DEFAULT_PROACTIVE_REFRESH_CREDENTIAL_TIME_GAP,
            clock_skew_gap: DEFAULT_CREDENTIAL_CLOCK_SKEW_GAP,
        }
    }
}

#[derive(Clone)]
pub(super) struct LastPresentedCredential {
    pub(super) credential: CredentialAndPurposeKey,
    pub(super) expires_at: TimestampInSeconds,
}

/// Credentials retriever for credentials located on a different node
#[derive(Clone)]
pub struct RemoteCredentialRetriever {
    ctx: Arc<Context>,
    transport: Arc<dyn Transport>,
    secure_channels: Arc<SecureChannels>,
    pub(super) issuer_info: RemoteCredentialRetrieverInfo,
    pub(super) subject: Identifier,
    scope: String,
    pub(super) timing_options: RemoteCredentialRetrieverTimingOptions,

    is_initialized: Arc<Mutex<bool>>,
    pub(super) last_presented_credential: Arc<RwLock<Option<LastPresentedCredential>>>,
    /// Subscribers addresses that we will notify when credential is refreshed
    pub(super) subscribers: Arc<RwLock<Vec<Address>>>,
}

impl RemoteCredentialRetriever {
    /// Create a new remote credential retriever
    pub fn new(
        ctx: Context,
        transport: Arc<dyn Transport>,
        secure_channels: Arc<SecureChannels>,
        issuer_info: RemoteCredentialRetrieverInfo,
        subject: Identifier,
        scope: String,
        timing_options: RemoteCredentialRetrieverTimingOptions,
    ) -> Self {
        debug!(
            "Creation of RemoteCredentialRetriever for: {}, authority: {}",
            subject, issuer_info.issuer
        );

        Self {
            ctx: Arc::new(ctx),
            transport,
            secure_channels,
            issuer_info,
            subject,
            scope,
            timing_options,
            is_initialized: Arc::new(Mutex::new(false)),
            last_presented_credential: Arc::new(RwLock::new(None)),
            subscribers: Default::default(),
        }
    }

    pub(super) async fn initialize_impl(&self) -> Result<()> {
        let mut is_initialized = self.is_initialized.lock().await;
        if *is_initialized {
            return Ok(());
        }

        debug!(
            "Initialization of RemoteCredentialRetriever for: {}, authority: {}",
            self.subject, self.issuer_info.issuer
        );

        let now = now()?;

        // Get a credential from the storage
        let last_presented_credential = match CachedCredentialRetriever::retrieve_impl(
            &self.issuer_info.issuer,
            &self.subject,
            &self.scope,
            now,
            self.secure_channels
                .identities
                .cached_credentials_repository(),
            self.timing_options.clock_skew_gap,
        )
        .await?
        {
            None => None,
            Some(last_presented_credential) => {
                let expires_at = last_presented_credential.get_expires_at()?;
                Some(LastPresentedCredential {
                    credential: last_presented_credential,
                    expires_at,
                })
            }
        };

        *self.last_presented_credential.write().unwrap() = last_presented_credential;

        let refresh_in = self.compute_refresh_duration(now, false);

        if !refresh_in.has_valid_credential {
            // We don't have a valid credential - refresh immediately and wait for the result
            debug!(
                "Creation of RemoteCredentialRetriever for: {}, authority: {} requires immediate credential refresh",
                self.subject, self.issuer_info.issuer
            );
            self.get_new_credential().await?;
        } else {
            // We still have a valid credential - schedule refresh in the background
            self.schedule_credentials_refresh_impl(refresh_in.duration, false);
        }

        *is_initialized = true;

        Ok(())
    }
}

struct RefreshDuration {
    duration: Duration,
    has_valid_credential: bool,
}

impl RemoteCredentialRetriever {
    fn compute_refresh_duration(&self, now: TimestampInSeconds, is_retry: bool) -> RefreshDuration {
        let last_presented_credential_expires_at = self
            .last_presented_credential
            .read()
            .unwrap()
            .as_ref()
            .map(|c| c.expires_at)
            .unwrap_or(now);

        let mut has_valid_credential = false;
        let refresh_in = if last_presented_credential_expires_at
            <= now + self.timing_options.clock_skew_gap
        {
            // Credential is considered expired. We already need to refresh.
            0.into()
        } else if last_presented_credential_expires_at
            <= now + self.timing_options.clock_skew_gap + self.timing_options.proactive_refresh_gap
        {
            // Credential is not expired, but it's already time to refresh it
            has_valid_credential = true;
            0.into()
        } else {
            // Credential is not expired, and will need refresh later
            last_presented_credential_expires_at
                - now
                - self.timing_options.clock_skew_gap
                - self.timing_options.proactive_refresh_gap
        };
        let refresh_in = Duration::from(refresh_in);

        let refresh_in = if is_retry {
            // Avoid too many request to the credential_retriever, the refresh can't be sooner than
            // self.min_credential_refresh_interval if it's a retry
            max(self.timing_options.min_refresh_interval, refresh_in)
        } else {
            refresh_in
        };

        RefreshDuration {
            duration: refresh_in,
            has_valid_credential,
        }
    }

    /// Schedule a DelayedEvent that will at specific point in time put a message
    /// into EncryptorWorker's own internal mailbox which it will use as a trigger to get a new
    /// credential and present it to the other side.
    fn schedule_credentials_refresh(&self, now: TimestampInSeconds, is_retry: bool) {
        let refresh_in = self.compute_refresh_duration(now, is_retry);

        self.schedule_credentials_refresh_impl(refresh_in.duration, is_retry);
    }

    async fn notify_subscribers(&self) -> Result<()> {
        let subscribers = self.subscribers.read().unwrap().clone();
        for subscriber in subscribers {
            match self.ctx.send(route![subscriber.clone()], ()).await {
                Ok(_) => {
                    debug!(
                        "Notified RemoteCredentialRetriever subscriber {}",
                        subscriber
                    )
                }
                Err(_err) => {
                    warn!(
                        "Error notifying RemoteCredentialRetriever subscriber {}",
                        subscriber
                    );
                }
            }
        }

        Ok(())
    }

    /// Schedule a DelayedEvent that will at specific point in time put a message
    /// into EncryptorWorker's own internal mailbox which it will use as a trigger to get a new
    /// credential and present it to the other side.
    fn schedule_credentials_refresh_impl(&self, refresh_in: Duration, is_retry: bool) {
        debug!(issuer=%self.issuer_info.issuer, is_retry,
            "Scheduling background credentials refresh in {} seconds",
            refresh_in.as_secs());
        self.request_new_credential_in_background(refresh_in, is_retry);
    }
}

impl RemoteCredentialRetriever {
    async fn get_new_credential(&self) -> Result<()> {
        debug!(subject=%self.subject, issuer=%self.issuer_info.issuer,
            "retrieving a new credential");
        let cache = self
            .secure_channels
            .identities
            .cached_credentials_repository();

        let client = SecureClient::new(
            self.secure_channels.clone(),
            None,
            self.transport.clone(),
            self.issuer_info.route.clone(),
            &self.issuer_info.issuer,
            &self.subject,
            self.timing_options.secure_channel_creation_timeout,
            self.timing_options.request_timeout,
        );

        let credential: CredentialAndPurposeKey = client
            .ask(
                &self.ctx,
                &self.issuer_info.service_address,
                Request::build(
                    self.issuer_info.request_method,
                    self.issuer_info.api_service_address.clone(),
                ),
            )
            .await?
            .success()?;

        let credential_data = credential.get_credential_data()?;
        let attributes = credential_data.get_attributes_display();
        let purpose_key_data = credential.purpose_key_attestation.get_attestation_data()?;
        info! {
            subject = %self.subject,
            %attributes,
            schema = %credential_data.subject_attributes.schema.0,
            created_at = %credential_data.created_at,
            expires_at = %credential_data.expires_at,
            "retrieved credential"
        }
        debug! {
            subject = %purpose_key_data.subject,
            created_at = %purpose_key_data.created_at,
            expires_at = %purpose_key_data.expires_at,
            "retrieved credential - purpose key attestation"
        }

        let credential_and_purpose_key_data = self
            .secure_channels
            .identities()
            .credentials()
            .credentials_verification()
            .verify_credential(
                Some(&self.subject),
                &[self.issuer_info.issuer.clone()],
                &credential,
            )
            .await?;
        let expires_at = credential_and_purpose_key_data.credential_data.expires_at;

        trace!("the retrieved credential is valid");

        *self.last_presented_credential.write().unwrap() = Some(LastPresentedCredential {
            credential: credential.clone(),
            expires_at,
        });

        let caching_res = cache
            .put(
                &self.subject,
                &self.issuer_info.issuer,
                &self.scope,
                expires_at,
                credential,
            )
            .await;

        if let Some(err) = caching_res.err() {
            error!(subject=%self.subject, issuer=%self.issuer_info.issuer, %err,
                "error caching credential");
        }

        self.notify_subscribers().await?;
        let now = now()?;

        self.schedule_credentials_refresh(now, false);

        Ok(())
    }

    fn request_new_credential_in_background(&self, wait: Duration, is_retry: bool) {
        let s = self.clone();
        ockam_node::spawn(async move {
            info!(issuer=%s.issuer_info.issuer, is_retry,
                "scheduled background credentials refresh in {} seconds",
                wait.as_secs());
            s.ctx
                .sleep_long_until(*now().unwrap() + wait.as_secs())
                .await;
            debug!(issuer=%s.issuer_info.issuer, is_retry,
                "executing background credentials refresh");
            if let Some(err) = s.get_new_credential().await.err() {
                error!(subject=%s.subject, is_retry, %err,
                    "error refreshing credential in the background");
                s.schedule_credentials_refresh(now().unwrap(), true);
            } else {
                debug!(issuer=%s.issuer_info.issuer, is_retry, "credentials refreshed");
            }
        });
    }
}
