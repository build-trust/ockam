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
    vault: Arc<dyn XXInitializedVault>,
    key_tracker: KeyTracker,
    nonce_tracker: NonceTracker,
}

impl Decryptor {
    pub fn new(key_id: KeyId, vault: Arc<dyn XXInitializedVault>) -> Self {
        Self {
            vault,
            key_tracker: KeyTracker::new(key_id, KEY_RENEWAL_INTERVAL),
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
            .aead_aes_gcm_decrypt(&key, &payload[8..], &nonce_buffer, &[])
            .await;

        if result.is_ok() {
            self.nonce_tracker = nonce_tracker;
            if let Some(key_to_delete) = self.key_tracker.update_key(key)? {
                self.vault.delete_ephemeral_secret(key_to_delete).await?;
            }
        }
        result
    }
}

struct KeyTracker {
    current_key: KeyId,
    number_of_rekeys: u64,
    max_rekeys_reached: bool,
    previous_key: Option<KeyId>,
    renewal_interval: u64,
}

impl KeyTracker {
    fn new(key_id: KeyId, renewal_interval: u64) -> Self {
        KeyTracker {
            current_key: key_id,
            number_of_rekeys: 0,
            max_rekeys_reached: false,
            previous_key: None,
            renewal_interval,
        }
    }
}

impl KeyTracker {
    /// The rekeying algorithm specifies a series of intervals of size self.renewal_interval
    /// where each interval corresponds to a set of contiguous nonces using the same key.
    ///
    /// This function returns the key corresponding to the current nonce.
    ///
    /// This is either:
    ///   - the current key if the nonce falls into the current interval
    ///   - the previous key if the nonce falls before the current interval
    ///   - nothing if the the nonce falls after the current interval -> this indicates that a new key must be created
    ///   - an error if
    ///      - if the the nonce falls before the previous interval
    ///      - if it the previous nonce but is not set
    ///      - we reached the maximum number of rekeyings
    fn get_key(&self, nonce: u64) -> Result<Option<KeyId>> {
        debug!(
            "The current number of rekeys is {}, the rekey interval is {}",
            self.current_key, self.renewal_interval
        );

        // for example 2 rekeys happened, renewal every 10 keys
        // current batch of nonces -> 20 to 29
        let current_interval_start = self.number_of_rekeys * self.renewal_interval;

        // if we reached the maximum number of rekeyings we stop operating on this secure channel
        if self.max_rekeys_reached {
            warn!("The maximum number of available rekeying operation has been reached. The last interval was starting at {} and the interval size is {}",
                current_interval_start, self.renewal_interval);
            return Err(IdentityError::InvalidNonce.into());
        };

        if nonce >= current_interval_start {
            let nonce_age = nonce - current_interval_start;
            // if the nonce falls in the current interval return the current key
            if nonce_age < self.renewal_interval {
                Ok(Some(self.current_key.clone()))
            }
            // if the nonce falls in the next interval
            // otherwise indicate that we need to create a new key
            else if nonce_age < self.renewal_interval * 2 {
                Ok(None)
            }
            // otherwise the nonce is too far ahead
            else {
                warn!("This nonce is too far in the future: {}", nonce);
                Err(IdentityError::InvalidNonce.into())
            }
        // else return the previous key (if there is one) if the nonce is not too old
        } else if current_interval_start - nonce <= self.renewal_interval {
            if let Some(previous) = self.previous_key.clone() {
                Ok(Some(previous))
            } else {
                warn!("There should be a previous key for this nonce: {}", nonce);
                Err(IdentityError::InvalidNonce.into())
            }
        } else {
            warn!("This nonce is too old: {}", nonce);
            Err(IdentityError::InvalidNonce.into())
        }
    }

    // Update the key if a key renewal happened
    fn update_key(&mut self, decryption_key: KeyId) -> Result<Option<KeyId>> {
        let mut key_to_delete = None;
        // if the key used for the decryption is not the current key nor the previous key
        // this means that a rekeying happened
        if decryption_key != self.current_key && Some(decryption_key.clone()) != self.previous_key {
            if let Some(previous) = self.previous_key.clone() {
                key_to_delete = Some(previous)
            }
            self.previous_key.replace(self.current_key.clone());
            self.current_key = decryption_key;
            if u64::MAX - self.number_of_rekeys * self.renewal_interval < self.renewal_interval {
                self.max_rekeys_reached = true;
            } else {
                self.number_of_rekeys += 1;
            }
        }
        Ok(key_to_delete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_key_first_interval() {
        let key_id = "key_id".to_string();
        let key_tracker = KeyTracker::new(key_id.clone(), 10);

        assert_eq!(key_tracker.get_key(0).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(5).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(9).unwrap(), Some(key_id));
        assert_eq!(
            key_tracker.get_key(10).unwrap(),
            None,
            "the next key must be created"
        );
        assert_eq!(
            key_tracker.get_key(20).ok(),
            None,
            "this nonce is too far in the future"
        );
        assert_eq!(
            key_tracker.get_key(u64::MAX).ok(),
            None,
            "this nonce is too far in the future"
        );
    }

    #[test]
    fn test_get_key_middle_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let key_tracker = KeyTracker {
            current_key: key_id.clone(),
            number_of_rekeys: 5,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id.clone()),
            renewal_interval: 10,
        };

        assert_eq!(
            key_tracker.get_key(0).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(30).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(39).ok(),
            None,
            "this nonce is too far in the past"
        );
        assert_eq!(
            key_tracker.get_key(40).unwrap(),
            Some(previous_key_id.clone())
        );
        assert_eq!(
            key_tracker.get_key(45).unwrap(),
            Some(previous_key_id.clone())
        );
        assert_eq!(key_tracker.get_key(49).unwrap(), Some(previous_key_id));
        assert_eq!(key_tracker.get_key(50).unwrap(), Some(key_id.clone()));
        assert_eq!(key_tracker.get_key(59).unwrap(), Some(key_id));
        assert_eq!(
            key_tracker.get_key(60).unwrap(),
            None,
            "the next key must be created"
        );
        assert_eq!(
            key_tracker.get_key(u64::MAX).ok(),
            None,
            "this nonce is too far in the future"
        );
    }

    #[test]
    fn test_get_key_last_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let key_tracker = KeyTracker {
            current_key: key_id,
            number_of_rekeys: 5,
            max_rekeys_reached: true,
            previous_key: Some(previous_key_id),
            renewal_interval: 10,
        };

        assert_eq!(
            key_tracker.get_key(0).ok(),
            None,
            "we reached the last interval already. The channel needs to be recreated"
        );
    }

    #[test]
    fn test_update_key() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let new_key_id = "new_key_id".to_string();
        let mut key_tracker = KeyTracker {
            current_key: key_id.clone(),
            number_of_rekeys: 5,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id.clone()),
            renewal_interval: 10,
        };

        assert_eq!(key_tracker.update_key(key_id.clone()).unwrap(), None);
        assert_eq!(
            key_tracker.update_key(previous_key_id.clone()).unwrap(),
            None
        );
        assert_eq!(
            key_tracker.update_key(new_key_id.clone()).unwrap(),
            Some(previous_key_id),
            "the previous key id must be returned in order to be deleted",
        );
        assert_eq!(key_tracker.current_key, new_key_id);
        assert_eq!(key_tracker.previous_key, Some(key_id));
    }

    #[test]
    fn test_update_key_on_last_interval() {
        let key_id = "key_id".to_string();
        let previous_key_id = "previous_key_id".to_string();
        let new_key_id = "new_key_id".to_string();
        let mut key_tracker = KeyTracker {
            current_key: key_id,
            number_of_rekeys: u64::MAX / 10 - 1,
            max_rekeys_reached: false,
            previous_key: Some(previous_key_id),
            renewal_interval: 10,
        };

        // this brings us to the last interval
        key_tracker.update_key(new_key_id).unwrap();
        assert!(
            !key_tracker.max_rekeys_reached,
            "the maximum number of rekeys is not yet reached"
        );

        // now there are no more intervals available
        let new_key_id_2 = "new_key_id_2".to_string();
        key_tracker.update_key(new_key_id_2).unwrap();
        assert!(
            key_tracker.max_rekeys_reached,
            "the maximum number of rekeys is reached now"
        );
    }
}
