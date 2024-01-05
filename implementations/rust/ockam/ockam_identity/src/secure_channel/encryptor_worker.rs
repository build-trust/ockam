use core::cmp::max;
use core::sync::atomic::{AtomicBool, Ordering};

use tracing::{debug, error, info};

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Decodable, Error, Route};
use ockam_core::{Any, Result, Routed, Worker};
use ockam_node::{Context, DelayedEvent};

use crate::models::{CredentialData, VersionedData};
use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::api::{EncryptionRequest, EncryptionResponse};
use crate::secure_channel::encryptor::Encryptor;
use crate::utils::now;
use crate::{
    ChangeHistoryRepository, Identifier, IdentityError, PlaintextPayloadMessage,
    RefreshCredentialsMessage, SecureChannelMessage, TimestampInSeconds, TrustContext,
};

pub(crate) struct EncryptorWorker {
    //for debug purposes only
    role: &'static str,
    addresses: Addresses,
    remote_route: Route,
    encryptor: Encryptor,
    my_identifier: Identifier,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    /// Expiration timestamp of the credential we presented (or the soonest if there are multiple)
    min_credential_expiration: Option<TimestampInSeconds>,
    /// The smallest interval of querying for a new credential from the credential retriever.
    /// Helps avoid situation when the query returns an error immediately or very fast.
    min_credential_refresh_interval: Duration,
    /// The time interval before the credential expiration when we'll ask the credential retriever
    /// for a new one
    refresh_credential_time_gap: Duration,
    credential_refresh_event: Option<DelayedEvent<()>>,
    // TODO: Should be CredentialsRetriever
    trust_context: Option<TrustContext>,

    should_send_close: Arc<AtomicBool>,
}

impl EncryptorWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        role: &'static str,
        addresses: Addresses,
        remote_route: Route,
        encryptor: Encryptor,
        my_identifier: Identifier,
        change_history_repository: Arc<dyn ChangeHistoryRepository>,
        min_credential_expiration: Option<TimestampInSeconds>,
        min_credential_refresh_interval: Duration,
        refresh_credential_time_gap: Duration,
        trust_context: Option<TrustContext>,
        should_send_close: Arc<AtomicBool>,
    ) -> Self {
        Self {
            role,
            addresses,
            remote_route,
            encryptor,
            my_identifier,
            change_history_repository,
            min_credential_expiration,
            min_credential_refresh_interval,
            refresh_credential_time_gap,
            credential_refresh_event: None,
            trust_context,
            should_send_close,
        }
    }

    /// Encrypt the message
    async fn encrypt(&mut self, ctx: &Context, msg: SecureChannelMessage) -> Result<Vec<u8>> {
        match self.encryptor.encrypt(&minicbor::to_vec(&msg)?).await {
            Ok(encrypted_payload) => Ok(encrypted_payload),
            // If encryption failed, that means we have some internal error,
            // and we may be in an invalid state, it's better to stop the Worker
            Err(err) => {
                let address = self.addresses.encryptor.clone();
                error!("Error while encrypting: {err} at: {address}");
                ctx.stop_worker(address).await?;
                Err(err)
            }
        }
    }

    async fn handle_encrypt_api(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Encrypt API {}",
            self.role, &self.addresses.encryptor
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = EncryptionRequest::decode(&msg.into_transport_message().payload)?;

        let mut should_stop = false;

        // Encrypt the message
        let response = match self.encryptor.encrypt(&request.0).await {
            Ok(encrypted_payload) => EncryptionResponse::Ok(encrypted_payload),
            // If encryption failed, that means we have some internal error,
            // and we may be in an invalid state, it's better to stop the Worker
            Err(err) => {
                should_stop = true;
                error!(
                    "Error while encrypting: {err} at: {}",
                    self.addresses.encryptor
                );
                EncryptionResponse::Err(err)
            }
        };

        // Send the reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.encryptor_api.clone())
            .await?;

        if should_stop {
            ctx.stop_worker(self.addresses.encryptor.clone()).await?;
        }

        Ok(())
    }

    async fn handle_encrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Encrypt {}",
            self.role, &self.addresses.encryptor
        );

        let mut onward_route = msg.onward_route();
        let return_route = msg.return_route();

        // Remove our address
        let _ = onward_route.step();

        let msg = PlaintextPayloadMessage {
            onward_route,
            return_route,
            payload: msg.into_transport_message().payload,
        };
        let msg = SecureChannelMessage::Payload(msg);

        let msg = self.encrypt(ctx, msg).await?;

        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            self.remote_route.clone(),
            msg,
            self.addresses.encryptor.clone(),
        )
        .await?;

        Ok(())
    }

    /// Asks credential retriever for a new credential and presents it to the other side, including
    /// the latest change_history
    async fn handle_refresh_credentials(&mut self, ctx: &<Self as Worker>::Context) -> Result<()> {
        debug!(
            "Started credentials refresh for {}",
            self.addresses.encryptor
        );

        let change_history = self
            .change_history_repository
            .get_change_history(&self.my_identifier)
            .await?
            .ok_or_else(|| {
                Error::new(
                    Origin::Api,
                    Kind::NotFound,
                    format!(
                        "no change history found for identifier {}",
                        self.my_identifier
                    ),
                )
            })?;

        let credential = if let Some(trust_context) = &self.trust_context {
            match trust_context.get_credential(ctx, &self.my_identifier).await {
                Ok(Some(credential)) => credential,
                // TODO: remove the duplication with the next case when reworking the trust contexts
                Ok(None) => {
                    info!(
                        "Credentials refresh failed for {} and is rescheduled in {} seconds",
                        self.addresses.encryptor,
                        self.min_credential_refresh_interval.as_secs()
                    );
                    // Will schedule a refresh in self.min_credential_refresh_interval
                    self.schedule_credentials_refresh(ctx, true).await?;
                    return Err(IdentityError::NoCredentialsSet)?;
                }
                Err(err) => {
                    info!(
                        "Credentials refresh failed for {} and is rescheduled in {} seconds",
                        self.addresses.encryptor,
                        self.min_credential_refresh_interval.as_secs()
                    );
                    // Will schedule a refresh in self.min_credential_refresh_interval
                    self.schedule_credentials_refresh(ctx, true).await?;
                    return Err(err);
                }
            }
        } else {
            return Err(IdentityError::NoCredentialsRetriever)?;
        };

        let versioned_data: VersionedData = minicbor::decode(&credential.credential.data)?;
        let data = CredentialData::get_data(&versioned_data)?;
        self.min_credential_expiration = Some(data.expires_at);

        let msg = RefreshCredentialsMessage {
            change_history,
            credentials: vec![credential],
        };
        let msg = SecureChannelMessage::RefreshCredentials(msg);

        let msg = self.encrypt(ctx, msg).await?;

        info!(
            "Sending credentials refresh for {}",
            self.addresses.encryptor
        );

        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            self.remote_route.clone(),
            msg,
            self.addresses.encryptor.clone(),
        )
        .await?;

        self.schedule_credentials_refresh(ctx, false).await?;

        Ok(())
    }

    async fn send_close_channel(&mut self, ctx: &Context) -> Result<()> {
        let msg = SecureChannelMessage::Close;

        // Encrypt the message
        let msg = self.encrypt(ctx, msg).await?;

        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            self.remote_route.clone(),
            msg,
            self.addresses.encryptor.clone(),
        )
        .await?;

        Ok(())
    }

    /// Schedule a DelayedEvent that will at specific point in time put a message
    /// into EncryptorWorker's own internal mailbox which it will use as a trigger to get a new
    /// credential and present it to the other side.
    async fn schedule_credentials_refresh(&mut self, ctx: &Context, is_retry: bool) -> Result<()> {
        let min_credential_expiration =
            if let Some(min_credential_expiration) = self.min_credential_expiration {
                min_credential_expiration
            } else {
                // Do nothing if there is no expiration
                return Ok(());
            };

        // Cancel the old event
        self.credential_refresh_event = None;

        let now = now()?;

        let duration = if min_credential_expiration < now + self.refresh_credential_time_gap {
            Duration::from_secs(0)
        } else {
            // Refresh in self.refresh_credential_time_gap before the expiration
            Duration::from_secs(
                *(min_credential_expiration
                    - self.refresh_credential_time_gap.as_secs().into()
                    - now),
            )
        };

        let duration = if is_retry {
            // Avoid too many request to the credential_retriever, the refresh can't be sooner than
            // self.min_credential_refresh_interval if it's a retry
            max(self.min_credential_refresh_interval, duration)
        } else {
            duration
        };

        debug!(
            "Scheduling credentials refresh for {} in {} seconds",
            self.addresses.encryptor,
            duration.as_secs()
        );
        let mut credential_refresh_event =
            DelayedEvent::create(ctx, self.addresses.encryptor_internal.clone(), ()).await?;
        credential_refresh_event.schedule(duration).await?;

        self.credential_refresh_event = Some(credential_refresh_event);

        Ok(())
    }
}

#[async_trait]
impl Worker for EncryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.schedule_credentials_refresh(ctx, false).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.addresses.encryptor {
            self.handle_encrypt(ctx, msg).await?;
        } else if msg_addr == self.addresses.encryptor_api {
            self.handle_encrypt_api(ctx, msg).await?;
        } else if msg_addr == self.addresses.encryptor_internal {
            self.handle_refresh_credentials(ctx).await?;
        } else {
            return Err(IdentityError::UnknownChannelMsgDestination)?;
        }

        Ok(())
    }

    async fn shutdown(&mut self, context: &mut Self::Context) -> Result<()> {
        let _ = context
            .stop_worker(self.addresses.decryptor_internal.clone())
            .await;
        if self.should_send_close.load(Ordering::Relaxed) {
            let _ = self.send_close_channel(context).await;
        }
        self.encryptor.shutdown().await
    }
}
