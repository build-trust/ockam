use core::sync::atomic::{AtomicBool, Ordering};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Any, Result, Routed, TransportMessage};
use ockam_core::{Decodable, LocalMessage};
use ockam_node::Context;

use crate::models::Identifier;
use crate::secure_channel::encryptor::{Encryptor, KEY_RENEWAL_INTERVAL};
use crate::secure_channel::handshake::handshake_state_machine::CommonStateMachine;
use crate::secure_channel::key_tracker::KeyTracker;
use crate::secure_channel::nonce_tracker::NonceTracker;
use crate::secure_channel::Addresses;
use crate::{
    DecryptionRequest, DecryptionResponse, Identities, IdentityError,
    IdentitySecureChannelLocalInfo, PlaintextPayloadMessage, RefreshCredentialsMessage,
    SecureChannelMessage, TrustContext,
};

use ockam_vault::{AeadSecretKeyHandle, VaultForSecureChannels};
use tracing::{debug, info, warn};

pub(crate) struct DecryptorHandler {
    //for debug purposes only
    pub(crate) role: &'static str,
    pub(crate) addresses: Addresses,
    pub(crate) their_identity_id: Identifier,
    pub(crate) decryptor: Decryptor,

    identities: Arc<Identities>,
    trust_context: Option<TrustContext>,
    should_send_close: Arc<AtomicBool>,
}

impl DecryptorHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identities: Arc<Identities>,
        trust_context: Option<TrustContext>,
        role: &'static str,
        addresses: Addresses,
        key: AeadSecretKeyHandle,
        vault: Arc<dyn VaultForSecureChannels>,
        their_identity_id: Identifier,
        should_send_close: Arc<AtomicBool>,
    ) -> Self {
        Self {
            role,
            addresses,
            their_identity_id,
            decryptor: Decryptor::new(key, vault),
            identities,
            trust_context,
            should_send_close,
        }
    }

    pub(crate) async fn handle_decrypt_api(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt API {}",
            self.role, &self.addresses.decryptor_remote
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = DecryptionRequest::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&request.0).await;

        let response = match decrypted_payload {
            Ok(payload) => DecryptionResponse::Ok(payload),
            Err(err) => DecryptionResponse::Err(err),
        };

        // Send reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.decryptor_api.clone())
            .await?;

        Ok(())
    }

    async fn handle_payload(
        &mut self,
        ctx: &mut Context,
        mut msg: PlaintextPayloadMessage,
    ) -> Result<()> {
        // Add encryptor hop in the return_route (instead of our address)
        msg.return_route
            .modify()
            .prepend(self.addresses.encryptor.clone());

        let transport_message =
            TransportMessage::v1(msg.onward_route, msg.return_route, msg.payload);

        // Mark message LocalInfo with IdentitySecureChannelLocalInfo,
        // replacing any pre-existing entries
        let local_info =
            IdentitySecureChannelLocalInfo::mark(vec![], self.their_identity_id.clone())?;

        let msg = LocalMessage::new(transport_message, local_info);

        match ctx
            .forward_from_address(msg, self.addresses.decryptor_internal.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    "{} forwarding decrypted message from {}",
                    err, &self.addresses.encryptor
                );
                Ok(())
            }
        }
    }

    async fn handle_close(&mut self, ctx: &mut Context) -> Result<()> {
        // Prevent sending another Close message
        self.should_send_close.store(false, Ordering::Relaxed);
        // Should be enough to stop the encryptor, since it will stop the decryptor
        ctx.stop_worker(self.addresses.encryptor.clone()).await
    }

    async fn handle_refresh_credentials(
        &mut self,
        _ctx: &mut Context,
        msg: RefreshCredentialsMessage,
    ) -> Result<()> {
        debug!(
            "Handling credentials refresh request for {}",
            self.addresses.decryptor_remote
        );

        CommonStateMachine::process_identity_payload_static(
            self.identities.clone(),
            None,
            self.trust_context.clone(),
            Some(self.their_identity_id.clone()),
            msg.change_history,
            msg.credentials,
            None,
        )
        .await?;

        info!(
            "Successfully handled credentials refresh request for {}",
            self.addresses.decryptor_remote
        );

        Ok(())
    }

    pub(crate) async fn handle_decrypt(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt {}",
            self.role, &self.addresses.decryptor_remote
        );

        // Decode raw payload binary
        let payload = msg.into_transport_message().payload;
        let payload = Vec::<u8>::decode(&payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&payload).await?;

        let msg: SecureChannelMessage = minicbor::decode(&decrypted_payload)?;

        match msg {
            SecureChannelMessage::Payload(msg) => self.handle_payload(ctx, msg).await?,
            SecureChannelMessage::RefreshCredentials(msg) => {
                self.handle_refresh_credentials(ctx, msg).await?
            }
            SecureChannelMessage::Close => self.handle_close(ctx).await?,
        };

        Ok(())
    }

    /// Remove the channel keys on shutdown
    pub(crate) async fn shutdown(&self) -> Result<()> {
        self.decryptor.shutdown().await
    }
}

pub(crate) struct Decryptor {
    vault: Arc<dyn VaultForSecureChannels>,
    key_tracker: KeyTracker,
    nonce_tracker: NonceTracker,
}

impl Decryptor {
    pub fn new(key: AeadSecretKeyHandle, vault: Arc<dyn VaultForSecureChannels>) -> Self {
        Self {
            vault,
            key_tracker: KeyTracker::new(key, KEY_RENEWAL_INTERVAL),
            nonce_tracker: NonceTracker::new(),
        }
    }

    /// Restore 12-byte nonce needed for AES GCM from 8 byte that we use for noise
    fn convert_nonce_from_small(b: &[u8]) -> Result<(u64, [u8; 12])> {
        let bytes: [u8; 8] = b.try_into().map_err(|_| IdentityError::InvalidNonce)?;

        let nonce = u64::from_be_bytes(bytes);

        Ok((nonce, Encryptor::convert_nonce_from_u64(nonce).1))
    }

    pub async fn decrypt(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        if payload.len() < 8 {
            return Err(IdentityError::InvalidNonce)?;
        }

        let (nonce, nonce_buffer) = Self::convert_nonce_from_small(&payload[..8])?;
        let nonce_tracker = self.nonce_tracker.mark(nonce)?;

        // get the key corresponding to the current nonce and
        // rekey if necessary
        let key = if let Some(key) = self.key_tracker.get_key(nonce)? {
            key
        } else {
            Encryptor::rekey(&self.vault, &self.key_tracker.current_key).await?
        };

        // to improve protection against connection disruption attacks, we want to validate the
        // message with a decryption _before_ committing to the new state
        let result = self
            .vault
            .aead_decrypt(&key, &payload[8..], &nonce_buffer, &[])
            .await;

        if result.is_ok() {
            self.nonce_tracker = nonce_tracker;
            if let Some(key_to_delete) = self.key_tracker.update_key(key)? {
                self.vault.delete_aead_secret_key(key_to_delete).await?;
            }
        }
        result
    }

    /// Remove the channel keys on shutdown
    pub(crate) async fn shutdown(&self) -> Result<()> {
        self.vault
            .delete_aead_secret_key(self.key_tracker.current_key.clone())
            .await?;
        if let Some(previous_key) = self.key_tracker.previous_key.clone() {
            self.vault.delete_aead_secret_key(previous_key).await?;
        };
        Ok(())
    }
}
