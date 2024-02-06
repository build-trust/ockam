use core::cmp::max;
use tracing::{debug, error, info, warn};

use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::{route, Address, Decodable, Encodable, Encoded, Message, Result};
use ockam_node::compat::asynchronous::Mutex;
use ockam_node::Context;

use crate::models::CredentialAndPurposeKey;
use crate::utils::now;
use crate::{CredentialIssuer, CredentialsCache, Identifier, IdentityError, TimestampInSeconds};

/// This is the default interval before a credential expiration when we'll query for
/// a new credential to avoid it expiring before we got a new one.
pub const DEFAULT_PROACTIVE_REFRESH_CREDENTIAL_TIME_GAP: TimestampInSeconds =
    TimestampInSeconds(60);

/// Default minimal interval before 2 refreshed in case we retry the refresh.
pub const DEFAULT_MIN_REFRESH_CREDENTIAL_INTERVAL: Duration = Duration::from_secs(10);

/// Start refresh in the background before it expires
pub const DEFAULT_CREDENTIAL_PROACTIVE_REFRESH_GAP: TimestampInSeconds = TimestampInSeconds(60);

/// Credential is considered already expired if it expires in less than this gap to account for a machine with a
/// wrong time
const DEFAULT_CREDENTIAL_CLOCK_SKEW_GAP: TimestampInSeconds = TimestampInSeconds(60);

/// Credentials refresher for credentials located on a different node
#[derive(Clone)]
pub struct CredentialRefresher {
    ctx: Arc<Context>,
    /// This issuer can issue credentials for a given subject
    credential_issuer: Arc<CredentialIssuer>,
    /// This is a cache for already retrieved credentials
    credentials_cache: Arc<CredentialsCache>,
    /// Identifier for which we want to refresh credentials
    subject: Identifier,
    /// Options used to tune the refresh behavior
    timing_options: RemoteCredentialRefresherTimingOptions,
    /// This mutex makes sure that we only set-up the initial refresh once
    is_initialized: Arc<Mutex<bool>>,
    /// Subscribers addresses that we will notify when credential is refreshed
    subscribers: Arc<RwLock<Vec<Address>>>,
}

impl CredentialRefresher {
    /// Create a new remote credential refresher
    pub fn new(
        ctx: Arc<Context>,
        credential_issuer: Arc<CredentialIssuer>,
        credentials_cache: Arc<CredentialsCache>,
        subject: Identifier,
        timing_options: RemoteCredentialRefresherTimingOptions,
    ) -> Self {
        debug!("Creation of RemoteCredentialRefresher for: {}", subject);

        Self {
            ctx,
            credential_issuer,
            credentials_cache,
            subject,
            timing_options,
            is_initialized: Arc::new(Mutex::new(false)),
            subscribers: Default::default(),
        }
    }

    /// Make sure that an initial valid credential is available
    pub async fn initialize(&self) -> Result<()> {
        let mut is_initialized = self.is_initialized.lock().await;
        if *is_initialized {
            return Ok(());
        }

        debug!(
            "Initialization of RemoteCredentialRefresher for: {}",
            self.subject
        );

        let refresh_in = self.compute_refresh_duration(now()?, false).await?;

        if !refresh_in.has_valid_credential {
            // We don't have a valid credential - refresh immediately and wait for the result
            debug!(
                "Creation of RemoteCredentialRefresher for: {} requires immediate credential refresh",
                self.subject
            );
            self.get_new_credential().await?;
        } else {
            // We still have a valid credential - schedule refresh in the background
            self.schedule_credentials_refresh_impl(refresh_in.duration, false)
                .await?;
        }

        *is_initialized = true;

        Ok(())
    }

    /// The subscribe method can be used by a worker to subscribe to a refresh credential.
    /// See [`Encryptor`]
    pub fn subscribe(&self, address: &Address) -> Result<()> {
        let mut subscribers = self.subscribers.write().unwrap();

        if subscribers.contains(address) {
            return Err(IdentityError::AddressAlreadySubscribedForThatCredentialRefresher)?;
        }

        subscribers.push(address.clone());

        Ok(())
    }

    /// The unsubscribe method is used to stop subscribing to credential refresh events
    pub fn unsubscribe(&self, address: &Address) -> Result<()> {
        let mut subscribers = self.subscribers.write().unwrap();

        if let Some(i) = subscribers.iter().position(|x| x == address) {
            subscribers.remove(i);
            Ok(())
        } else {
            Err(IdentityError::AddressIsNotSubscribedForThatCredentialRefresher)?
        }
    }
}

struct RefreshDuration {
    duration: Duration,
    has_valid_credential: bool,
}

impl CredentialRefresher {
    async fn compute_refresh_duration(
        &self,
        now: TimestampInSeconds,
        is_retry: bool,
    ) -> Result<RefreshDuration> {
        let expires_at = match self
            .credentials_cache
            .get_credential(self.credential_issuer.issuer(), &self.subject)
            .await
        {
            Ok(c) => c.get_expires_at()?,
            _ => now,
        };

        let mut has_valid_credential = false;
        let refresh_in = if expires_at <= now + self.timing_options.clock_skew_gap {
            // Credential is considered expired. We already need to refresh.
            0.into()
        } else if expires_at
            <= now + self.timing_options.clock_skew_gap + self.timing_options.proactive_refresh_gap
        {
            // Credential is not expired, but it's already time to refresh it
            has_valid_credential = true;
            0.into()
        } else {
            // Credential is not expired, and will need refresh later
            expires_at
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

        Ok(RefreshDuration {
            duration: refresh_in,
            has_valid_credential,
        })
    }

    /// Schedule a DelayedEvent that will at specific point in time put a message
    /// into EncryptorWorker's own internal mailbox which it will use as a trigger to get a new
    /// credential and present it to the other side.
    async fn schedule_credentials_refresh(
        &self,
        now: TimestampInSeconds,
        is_retry: bool,
    ) -> Result<()> {
        let refresh_in = self.compute_refresh_duration(now, is_retry).await?;
        self.schedule_credentials_refresh_impl(refresh_in.duration, is_retry)
            .await
    }

    async fn notify_subscribers(
        &self,
        credential_and_purpose_key: CredentialAndPurposeKey,
    ) -> Result<()> {
        let subscribers = self.subscribers.read().unwrap().clone();
        for subscriber in subscribers {
            match self
                .ctx
                .send(
                    route![subscriber.clone()],
                    CredentialAndPurposeKeyMessage(credential_and_purpose_key.clone()),
                )
                .await
            {
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
    async fn schedule_credentials_refresh_impl(
        &self,
        refresh_in: Duration,
        is_retry: bool,
    ) -> Result<()> {
        let is_retry_str = if is_retry { " retry " } else { " " };
        info!(
            "Scheduling background credentials refresh{} in {} seconds",
            is_retry_str,
            refresh_in.as_secs()
        );
        self.request_new_credential_in_background(refresh_in, is_retry);
        Ok(())
    }
}

impl CredentialRefresher {
    async fn get_new_credential(&self) -> Result<()> {
        let credential_and_purpose_key = self
            .credential_issuer
            .renew_credential(&self.subject)
            .await?;
        self.notify_subscribers(credential_and_purpose_key).await?;
        self.schedule_credentials_refresh(now()?, false).await
    }

    fn request_new_credential_in_background(&self, wait: Duration, is_retry: bool) {
        let s = self.clone();
        ockam_node::spawn(async move {
            let is_retry_str = if is_retry { " retry " } else { " " };
            info!(
                "Scheduled background credentials refresh {} in {} seconds",
                is_retry_str,
                wait.as_secs()
            );
            let now = now().unwrap();

            s.ctx.sleep_long_until(*now + wait.as_secs()).await;
            info!("Executing background credentials refresh {}", is_retry_str,);
            let res = s.get_new_credential().await;

            if let Some(err) = res.err() {
                error!(
                    "Error refreshing credential for {} in the background: {}",
                    s.subject, err
                );
                if let Err(e) = s.schedule_credentials_refresh(now, true).await {
                    error!(
                        "Error scheduling a credential refresh for {}: {}",
                        s.subject, e
                    );
                }
            };
        });
    }
}

/// Timing options for refreshing remote credentials
#[derive(Debug, Clone, Copy)]
pub struct RemoteCredentialRefresherTimingOptions {
    /// Minimum interval before refresh requests to the Authority node
    pub min_refresh_interval: Duration,
    /// Time gap used to request a new credential before the old one actually expires
    pub proactive_refresh_gap: TimestampInSeconds,
    /// Time gap used to consider credential expired before its actual expiration
    /// to account for time errors on different machines
    pub clock_skew_gap: TimestampInSeconds,
}

impl Default for RemoteCredentialRefresherTimingOptions {
    fn default() -> Self {
        Self {
            min_refresh_interval: DEFAULT_MIN_REFRESH_CREDENTIAL_INTERVAL,
            proactive_refresh_gap: DEFAULT_PROACTIVE_REFRESH_CREDENTIAL_TIME_GAP,
            clock_skew_gap: DEFAULT_CREDENTIAL_CLOCK_SKEW_GAP,
        }
    }
}

/// This message is sent to an Encryptor when a refreshed credential is available
#[derive(Clone, Debug)]
pub struct CredentialAndPurposeKeyMessage(pub(crate) CredentialAndPurposeKey);

impl Encodable for CredentialAndPurposeKeyMessage {
    fn encode(&self) -> Result<Encoded> {
        self.0.encode_as_cbor_bytes()
    }
}

impl Decodable for CredentialAndPurposeKeyMessage {
    fn decode(e: &[u8]) -> Result<Self> {
        Ok(CredentialAndPurposeKeyMessage(
            CredentialAndPurposeKey::decode_from_cbor_bytes(e)?,
        ))
    }
}

impl Message for CredentialAndPurposeKeyMessage {}
