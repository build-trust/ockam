use core::sync::atomic::{AtomicBool, Ordering};

use tracing::{debug, error, info, warn};
use tracing_attributes::instrument;

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Decodable, Encodable, Error, LocalMessage, Route};
use ockam_core::{Any, Result, Routed, Worker};
use ockam_node::Context;

use crate::models::CredentialAndPurposeKey;
use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::api::{EncryptionRequest, EncryptionResponse};
use crate::secure_channel::encryptor::Encryptor;
use crate::{
    ChangeHistoryRepository, CredentialAndPurposeKeyMessage, CredentialRefresher, Identifier,
    IdentityError, PlaintextPayloadMessage, RefreshCredentialsMessage, SecureChannelMessage,
};

#[derive(Debug, Clone)]
pub(crate) struct SecureChannelSharedState {
    /// Allows Decryptor to flag that we're closing the channel because we received a Close message from the other side,
    /// therefore, we don't need to send that message again to the other side
    pub(crate) should_send_close: Arc<AtomicBool>,
}

pub(crate) struct EncryptorWorker {
    role: &'static str, // For debug purposes only
    addresses: Addresses,
    remote_route: Route,
    encryptor: Encryptor,
    my_identifier: Identifier,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    credential_refresher: Option<Arc<CredentialRefresher>>,
    last_presented_credential: Option<CredentialAndPurposeKey>,
    shared_state: SecureChannelSharedState,
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
        credential_refresher: Option<Arc<CredentialRefresher>>,
        last_presented_credential: Option<CredentialAndPurposeKey>,
        shared_state: SecureChannelSharedState,
    ) -> Self {
        Self {
            role,
            addresses,
            remote_route,
            encryptor,
            my_identifier,
            change_history_repository,
            credential_refresher,
            last_presented_credential,
            shared_state,
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

        let msg = PlaintextPayloadMessage {
            onward_route,
            return_route,
            payload: msg.payload().to_vec(),
        };
        let msg = SecureChannelMessage::Payload(msg);

        let msg = self.encrypt(ctx, msg).await?;

        let payload = msg.encode()?;
        // Decryptor doesn't need the return_route since it has `self.remote_route` as well
        let msg = LocalMessage::new()
            .with_payload(payload)
            .with_onward_route(self.remote_route.clone());

        // Send the message to the decryptor on the other side
        ctx.forward_from_address(msg, self.addresses.encryptor.clone())
            .await?;

        Ok(())
    }

    /// Asks credential retriever for a new credential and presents it to the other side, including
    /// the latest change_history
    #[instrument(skip_all)]
    async fn handle_refresh_credentials(
        &mut self,
        ctx: &<Self as Worker>::Context,
        credential_and_purpose_key: CredentialAndPurposeKey,
    ) -> Result<()> {
        debug!(
            "Started credentials refresh for {}",
            self.addresses.encryptor
        );

        if Some(&credential_and_purpose_key) == self.last_presented_credential.as_ref() {
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
            credentials: vec![credential_and_purpose_key.clone()],
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

        self.last_presented_credential = Some(credential_and_purpose_key);

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
}

#[async_trait]
impl Worker for EncryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        if let Some(remote_credential_refresher) = &self.credential_refresher {
            remote_credential_refresher.subscribe(&self.addresses.encryptor_internal)?;
        }
        Ok(())
    }

    #[instrument(skip_all, name = "EncryptorWorker::handle_message", fields(worker = %ctx.address()))]
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
            let credential_and_purpose_key = CredentialAndPurposeKeyMessage::decode(msg.payload())?;
            self.handle_refresh_credentials(ctx, credential_and_purpose_key.0)
                .await?;
        } else {
            return Err(IdentityError::UnknownChannelMsgDestination)?;
        }

        Ok(())
    }

    #[instrument(skip_all, name = "EncryptorWorker::shutdown")]
    async fn shutdown(&mut self, context: &mut Self::Context) -> Result<()> {
        if let Some(remote_credential_refresher) = &self.credential_refresher {
            remote_credential_refresher.unsubscribe(&self.addresses.encryptor_internal)?;
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
