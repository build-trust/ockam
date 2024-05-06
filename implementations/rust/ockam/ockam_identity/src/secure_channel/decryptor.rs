use core::sync::atomic::Ordering;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{route, Any, Result, Route, Routed};
use ockam_core::{Decodable, LocalMessage};
use ockam_node::Context;

use crate::models::Identifier;
use crate::secure_channel::encryptor::{Encryptor, KEY_RENEWAL_INTERVAL};
use crate::secure_channel::handshake::handshake_state_machine::CommonStateMachine;
use crate::secure_channel::key_tracker::KeyTracker;
use crate::secure_channel::nonce_tracker::NonceTracker;
use crate::secure_channel::{Addresses, Role};
use crate::{
    DecryptionRequest, DecryptionResponse, Identities, IdentityError,
    IdentitySecureChannelLocalInfo, Nonce, PlaintextPayloadMessage, RefreshCredentialsMessage,
    SecureChannelMessage,
};

use crate::secure_channel::encryptor_worker::SecureChannelSharedState;
use crate::secure_channel::payload_collector::PayloadCollector;
use ockam_core::errcode::{Kind, Origin};
use ockam_vault::{AeadSecretKeyHandle, VaultForSecureChannels};
use tracing::{debug, info, trace, warn};
use tracing_attributes::instrument;
use uuid::Uuid;

pub(crate) struct DecryptorHandler {
    //for debug purposes only
    pub(crate) role: Role,
    pub(crate) addresses: Addresses,
    pub(crate) their_identity_id: Identifier,
    pub(crate) decryptor: Decryptor,

    identities: Arc<Identities>,
    authority: Option<Identifier>,
    shared_state: SecureChannelSharedState,
    payload_collector: PayloadCollector,
}

impl DecryptorHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identities: Arc<Identities>,
        authority: Option<Identifier>,
        role: Role,
        addresses: Addresses,
        key: AeadSecretKeyHandle,
        vault: Arc<dyn VaultForSecureChannels>,
        their_identity_id: Identifier,
        shared_state: SecureChannelSharedState,
    ) -> Self {
        Self {
            role,
            addresses,
            their_identity_id,
            decryptor: Decryptor::new(key, vault),
            identities,
            authority,
            shared_state,
            payload_collector: PayloadCollector::new(),
        }
    }

    #[instrument(skip_all)]
    pub(crate) async fn handle_decrypt_api(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
    ) -> Result<()> {
        trace!(
            "SecureChannel {} received Decrypt API {}",
            self.role,
            &self.addresses.decryptor_remote
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = DecryptionRequest::decode(msg.payload())?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&request.0).await;

        let response = match decrypted_payload {
            Ok((payload, _nonce)) => DecryptionResponse::Ok(payload),
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
        mut msg: PlaintextPayloadMessage<'_>,
        nonce: Nonce,
        encrypted_msg_return_route: Route,
    ) -> Result<()> {
        if !self.role.is_initiator() {
            let mut remote_route = self.shared_state.remote_route.write().unwrap();
            // Only overwrite if we know that's the latest address
            if remote_route.last_nonce < nonce {
                let their_decryptor_address = remote_route.route.recipient()?;
                remote_route.route = route![encrypted_msg_return_route, their_decryptor_address];
                remote_route.last_nonce = nonce;
            }
        }

        // Add encryptor hop in the return_route (instead of our address)
        msg.return_route
            .modify()
            .prepend(self.addresses.encryptor.clone());

        // Mark message LocalInfo with IdentitySecureChannelLocalInfo,
        // replacing any pre-existing entries
        let local_info =
            IdentitySecureChannelLocalInfo::mark(vec![], self.their_identity_id.clone())?;

        let msg = LocalMessage::new()
            .with_onward_route(msg.onward_route)
            .with_return_route(msg.return_route)
            .with_payload(msg.payload.to_vec())
            .with_local_info(local_info);

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

    /// This handler receives a payload part and
    /// updates the payload collector.
    /// If all the parts of a given payload (as determined by the payload uuid) have been received
    /// then the full payload is returned by the collector and the payload is handled
    /// via the handle_payload method.
    async fn handle_payload_part<'a>(
        &mut self,
        ctx: &mut Context,
        nonce: Nonce,
        encrypted_msg_return_route: Route,
        part: PlaintextPayloadMessage<'a>,
        payload_uuid: Uuid,
        current_part_number: u32,
        total_number_of_parts: u32,
    ) -> Result<()> {
        if let Some(payload) = self
            .payload_collector
            .update(
                payload_uuid,
                &part.onward_route,
                &part.return_route,
                part.payload.to_vec(),
                current_part_number,
                total_number_of_parts,
            )
            .await?
        {
            let message = PlaintextPayloadMessage {
                onward_route: part.onward_route.clone(),
                return_route: part.return_route.clone(),
                payload: payload.as_slice(),
            };
            self.handle_payload(ctx, message, nonce, encrypted_msg_return_route)
                .await
        } else {
            Ok(())
        }
    }

    async fn handle_close(&mut self, ctx: &mut Context) -> Result<()> {
        // Prevent sending another Close message
        self.shared_state
            .should_send_close
            .store(false, Ordering::Relaxed);
        // Should be enough to stop the encryptor, since it will stop the decryptor
        ctx.stop_worker(self.addresses.encryptor.clone()).await
    }

    async fn handle_refresh_credentials(
        &mut self,
        _ctx: &mut Context,
        msg: RefreshCredentialsMessage,
    ) -> Result<()> {
        debug!(
            "Handling credentials refresh for {}",
            self.addresses.decryptor_remote
        );

        CommonStateMachine::process_identity_payload_static(
            self.identities.clone(),
            None,
            self.authority.clone(),
            Some(self.their_identity_id.clone()),
            msg.change_history,
            msg.credentials,
            None,
        )
        .await?;

        info!(
            "Successfully handled credentials refresh for {}",
            self.addresses.decryptor_remote
        );

        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn handle_decrypt(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Any>,
    ) -> Result<()> {
        trace!(
            "SecureChannel {} received Decrypt {}",
            self.role,
            &self.addresses.decryptor_remote
        );

        let encrypted_msg_return_route = msg.return_route();

        // Decode raw payload binary
        let payload = msg.into_payload();
        let payload =
            ockam_core::bare::read_slice(payload.as_slice(), &mut 0).ok_or_else(|| {
                ockam_core::Error::new(Origin::Transport, Kind::Protocol, "Invalid message")
            })?;

        // Decrypt the binary
        let (decrypted_payload, nonce) = self.decryptor.decrypt(payload).await?;
        let decrypted_msg: SecureChannelMessage = minicbor::decode(&decrypted_payload)?;
        match decrypted_msg {
            SecureChannelMessage::Payload(decrypted_msg) => {
                self.handle_payload(ctx, decrypted_msg, nonce, encrypted_msg_return_route)
                    .await?
            }
            SecureChannelMessage::PayloadPart {
                part,
                payload_uuid,
                current_part_number,
                total_number_of_parts,
            } => {
                self.handle_payload_part(
                    ctx,
                    nonce,
                    encrypted_msg_return_route,
                    part.clone(),
                    payload_uuid.into(),
                    current_part_number,
                    total_number_of_parts,
                )
                .await?
            }
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

    #[instrument(skip_all)]
    pub async fn decrypt(&mut self, payload: &[u8]) -> Result<(Vec<u8>, Nonce)> {
        if payload.len() < 8 {
            return Err(IdentityError::InvalidNonce)?;
        }

        let nonce = Nonce::try_from(&payload[..8])?;
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
            .aead_decrypt(&key, &payload[8..], &nonce.to_aes_gcm_nonce(), &[])
            .await;

        if result.is_ok() {
            self.nonce_tracker = nonce_tracker;
            if let Some(key_to_delete) = self.key_tracker.update_key(key)? {
                self.vault.delete_aead_secret_key(key_to_delete).await?;
            }
        }

        result.map(|payload| (payload, nonce))
    }

    /// Remove the channel keys on shutdown
    #[instrument(skip_all)]
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
