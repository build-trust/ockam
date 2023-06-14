use crate::secure_channel::encryptor::{Encryptor, KEY_RENEWAL_INTERVAL};
use crate::secure_channel::nonce_tracker::NonceTracker;
use crate::secure_channel::Addresses;
use crate::XXInitializedVault;
use crate::{
    DecryptionRequest, DecryptionResponse, IdentityError, IdentityIdentifier,
    IdentitySecureChannelLocalInfo,
};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Any, Result, Routed, TransportMessage};
use ockam_core::{Decodable, LocalMessage};
use ockam_node::Context;
use ockam_vault::KeyId;
use tracing::debug;
use tracing::warn;

pub(crate) struct DecryptorHandler {
    //for debug purposes only
    pub(crate) role: &'static str,
    pub(crate) addresses: Addresses,
    pub(crate) their_identity_id: IdentityIdentifier,
    pub(crate) decryptor: Decryptor,
}

impl DecryptorHandler {
    pub fn new(
        role: &'static str,
        addresses: Addresses,
        key: KeyId,
        vault: Arc<dyn XXInitializedVault>,
        their_identity_id: IdentityIdentifier,
    ) -> Self {
        Self {
            role,
            addresses,
            their_identity_id,
            decryptor: Decryptor::new(key, vault),
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
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&payload).await?;

        // Encrypted data should be a TransportMessage
        let mut transport_message = TransportMessage::decode(&decrypted_payload)?;

        // Add encryptor hop in the return_route (instead of our address)
        transport_message
            .return_route
            .modify()
            .prepend(self.addresses.encryptor.clone());

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
}

pub(crate) struct Decryptor {
    current_key: KeyId,
    current_key_nonce: u64,

    previous_key: Option<KeyId>,
    vault: Arc<dyn XXInitializedVault>,
    nonce_tracker: NonceTracker,
}

impl Decryptor {
    pub fn new(key: KeyId, vault: Arc<dyn XXInitializedVault>) -> Self {
        Self {
            current_key: key,
            current_key_nonce: 0,
            previous_key: None,
            vault,
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
            return Err(IdentityError::InvalidNonce.into());
        }

        let (nonce, nonce_buffer) = Self::convert_nonce_from_small(&payload[..8])?;

        let nonce_tracker = self.nonce_tracker.mark(nonce)?;

        // to improve protection against connection disruption attacks, we want to validate the
        // message with a decryption _before_ committing to the new state

        if nonce >= self.current_key_nonce + KEY_RENEWAL_INTERVAL {
            // we need to rekey
            let new_key = Encryptor::rekey(&self.vault, &self.current_key).await?;
            let new_key_nonce = nonce - nonce % KEY_RENEWAL_INTERVAL;

            let result = self
                .vault
                .aead_aes_gcm_decrypt(&new_key, &payload[8..], &nonce_buffer, &[])
                .await;

            if result.is_ok() {
                if let Some(previous_key) = self.previous_key.replace(self.current_key.clone()) {
                    self.vault.delete_ephemeral_secret(previous_key).await?;
                }

                self.nonce_tracker = nonce_tracker;
                self.current_key = new_key;
                self.current_key_nonce = new_key_nonce;
            }

            result
        } else {
            let key = if nonce >= self.current_key_nonce {
                &self.current_key
            } else if let Some(key) = &self.previous_key {
                key
            } else {
                // shouldn't happen since nonce_tracker should reject such messages
                warn!("invalid nonce for previous key");
                return Err(IdentityError::InvalidNonce.into());
            };

            let result = self
                .vault
                .aead_aes_gcm_decrypt(key, &payload[8..], &nonce_buffer, &[])
                .await;

            if result.is_ok() {
                self.nonce_tracker = nonce_tracker;
            }

            result
        }
    }
}
