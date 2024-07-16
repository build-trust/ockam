use core::sync::atomic::{AtomicBool, Ordering};

use tracing::{debug, error, info, warn};
use tracing_attributes::instrument;

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    async_trait, route, CowBytes, Decodable, Error, LocalMessage, NeutralMessage, Route,
};
use ockam_core::{Any, Result, Routed, Worker};
use ockam_node::Context;

use crate::models::CredentialAndPurposeKey;
use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::api::{EncryptionRequest, EncryptionResponse};
use crate::secure_channel::encryptor::{Encryptor, SIZE_OF_ENCRYPT_OVERHEAD};
use crate::{
    ChangeHistoryRepository, CredentialRetriever, Identifier, IdentityError, Nonce,
    PlaintextPayloadMessage, RefreshCredentialsMessage, SecureChannelMessage,
    SecureChannelPaddedMessage,
};

/// Wrap last received (during successful decryption) nonce and current route to the remote in a
/// struct to allow shared access to it. That allows updating it either by calling
/// [`SecureChannel::update_remote_node_route`] on the initiator side, or when we receive a message
/// with an updated `return_route` on the responder side.
/// The route points to the decryptor on the other side.
#[derive(Debug, Clone)]
pub(crate) struct RemoteRoute {
    pub(crate) route: Route,
    pub(crate) last_nonce: Nonce,
}

impl RemoteRoute {
    pub fn create() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            route: route![],
            last_nonce: 0.into(),
        }))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SecureChannelSharedState {
    /// Route to the decryptor on the other side. Can be updated from the initiator side by calling
    /// [`SecureChannel::update_remote_node_route`] or will be updated under the hood for responder
    /// side upon receiving a message with an updated `return_route`
    pub(crate) remote_route: Arc<RwLock<RemoteRoute>>,
    /// Allows Decryptor to flag that we're closing the channel because we received a Close message from the other side,
    /// therefore, we don't need to send that message again to the other side
    pub(crate) should_send_close: Arc<AtomicBool>,
}

pub(crate) struct EncryptorWorker {
    role: &'static str, // For debug purposes only
    key_exchange_only: bool,
    addresses: Addresses,
    encryptor: Encryptor,
    my_identifier: Identifier,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    credential_retriever: Option<Arc<dyn CredentialRetriever>>,
    last_presented_credential: Option<CredentialAndPurposeKey>,
    shared_state: SecureChannelSharedState,
}

impl EncryptorWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        role: &'static str,
        key_exchange_only: bool,
        addresses: Addresses,
        encryptor: Encryptor,
        my_identifier: Identifier,
        change_history_repository: Arc<dyn ChangeHistoryRepository>,
        credential_retriever: Option<Arc<dyn CredentialRetriever>>,
        last_presented_credential: Option<CredentialAndPurposeKey>,
        shared_state: SecureChannelSharedState,
    ) -> Self {
        Self {
            role,
            key_exchange_only,
            addresses,
            encryptor,
            my_identifier,
            change_history_repository,
            credential_retriever,
            last_presented_credential,
            shared_state,
        }
    }

    /// Encrypt the message
    async fn encrypt(
        &mut self,
        ctx: &Context,
        msg: SecureChannelPaddedMessage<'_>,
    ) -> Result<Vec<u8>> {
        let payload = ockam_core::cbor_encode_preallocate(&msg)?;
        let mut destination = Vec::with_capacity(SIZE_OF_ENCRYPT_OVERHEAD + payload.len());

        match self.encryptor.encrypt(&mut destination, &payload).await {
            Ok(()) => Ok(destination),
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

    #[instrument(skip_all)]
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
        let request = EncryptionRequest::decode(msg.payload())?;

        let mut should_stop = false;
        let mut encrypted_payload = Vec::new();

        // Encrypt the message
        let response = match self
            .encryptor
            .encrypt(&mut encrypted_payload, &request.0)
            .await
        {
            Ok(()) => EncryptionResponse::Ok(encrypted_payload),
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

    #[instrument(skip_all)]
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

        let payload = CowBytes::from(msg.into_payload());
        let msg = PlaintextPayloadMessage {
            onward_route,
            return_route,
            payload,
        };

        let msg = SecureChannelMessage::Payload(msg);
        let msg = Self::add_padding(msg);

        let payload = self.encrypt(ctx, msg).await?;

        let remote_route = self.shared_state.remote_route.read().unwrap().route.clone();
        // Decryptor doesn't need the return_route since it has `self.remote_route` as well
        let msg = LocalMessage::new()
            .with_payload(payload)
            .with_onward_route(remote_route);

        // Send the message to the decryptor on the other side
        ctx.forward_from_address(msg, self.addresses.encryptor.clone())
            .await?;

        Ok(())
    }

    /// Asks credential retriever for a new credential and presents it to the other side, including
    /// the latest change_history
    #[instrument(skip_all)]
    async fn handle_refresh_credentials(&mut self, ctx: &<Self as Worker>::Context) -> Result<()> {
        debug!(
            "Started credentials refresh for {}",
            self.addresses.encryptor
        );

        let credential_retriever = match &self.credential_retriever {
            Some(credential_retriever) => credential_retriever,
            None => return Err(IdentityError::NoCredentialRetriever)?,
        };

        let credential = match credential_retriever.retrieve().await {
            Ok(credential) => credential,
            Err(err) => {
                error!(
                    "Credentials refresh failed for {} with error={}",
                    self.addresses.encryptor, err,
                );
                return Err(err);
            }
        };

        if Some(&credential) == self.last_presented_credential.as_ref() {
            // Credential hasn't actually changed
            warn!(
                "Credentials refresh for {} cancelled since credential hasn't changed",
                self.addresses.encryptor
            );
            return Ok(());
        }

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

        let msg = RefreshCredentialsMessage {
            change_history,
            credentials: vec![credential.clone()],
        };
        let msg = SecureChannelMessage::RefreshCredentials(msg);
        let msg = Self::add_padding(msg);

        let msg = self.encrypt(ctx, msg).await?;

        info!(
            "Sending credentials refresh for {}",
            self.addresses.encryptor
        );

        let remote_route = self.shared_state.remote_route.read().unwrap().route.clone();
        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            remote_route,
            NeutralMessage::from(msg),
            self.addresses.encryptor.clone(),
        )
        .await?;

        self.last_presented_credential = Some(credential);

        Ok(())
    }

    async fn send_close_channel(&mut self, ctx: &Context) -> Result<()> {
        let msg = SecureChannelMessage::Close;
        let msg = Self::add_padding(msg);

        // Encrypt the message
        let msg = self.encrypt(ctx, msg).await?;

        let remote_route = self.shared_state.remote_route.read().unwrap().route.clone();
        // Send the message to the decryptor on the other side
        ctx.send_from_address(
            remote_route,
            NeutralMessage::from(msg),
            self.addresses.encryptor.clone(),
        )
        .await?;

        Ok(())
    }

    fn add_padding(msg: SecureChannelMessage) -> SecureChannelPaddedMessage {
        // Naїve padding of 0 to 255 zeros
        // let padding_length: u8 = ockam_core::compat::rand::random();
        // let padding = vec![0u8; padding_length as usize];

        let padding = vec![];

        SecureChannelPaddedMessage {
            message: msg,
            padding: padding.into(),
        }
    }
}

#[async_trait]
impl Worker for EncryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        if let Some(credential_retriever) = &self.credential_retriever {
            credential_retriever.subscribe(&self.addresses.encryptor_internal)?;
        }

        Ok(())
    }

    #[instrument(skip_all, name = "EncryptorWorker::handle_message", fields(worker = % ctx.address()))]
    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if self.key_exchange_only {
            if msg_addr == self.addresses.encryptor_api {
                self.handle_encrypt_api(ctx, msg).await?;
            } else {
                return Err(IdentityError::UnknownChannelMsgDestination)?;
            }
        } else if msg_addr == self.addresses.encryptor {
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

    #[instrument(skip_all, name = "EncryptorWorker::shutdown")]
    async fn shutdown(&mut self, context: &mut Self::Context) -> Result<()> {
        if let Some(credential_retriever) = &self.credential_retriever {
            credential_retriever.unsubscribe(&self.addresses.encryptor_internal)?;
        }

        let _ = context
            .stop_worker(self.addresses.decryptor_internal.clone())
            .await;
        if self.shared_state.should_send_close.load(Ordering::Relaxed) {
            let _ = self.send_close_channel(context).await;
        }
        self.encryptor.shutdown().await
    }
}
