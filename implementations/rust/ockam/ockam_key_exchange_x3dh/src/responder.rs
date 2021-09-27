use crate::{PreKeyBundle, Signature, X3DHError, X3dhVault, CSUITE};
use arrayref::array_ref;
use ockam_core::async_trait::async_trait;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::Result;
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_SECRET_LENGTH,
};

#[derive(Debug)]
enum ResponderState {
    /// Expect an enrollment message from this EIK
    HandleInitiatorKeys,
    /// Create a PreKey Bundle
    SendBundle,
    /// Done
    Done,
}

/// The responder of X3DH creates a prekey bundle that can be used to establish a shared
/// secret key with another party that can use
pub struct Responder<V: X3dhVault> {
    identity_key: Option<Secret>,
    signed_prekey: Option<Secret>,
    one_time_prekey: Option<Secret>,
    state: ResponderState,
    vault: V,
    completed_key_exchange: Option<CompletedKeyExchange>,
}

impl<V: X3dhVault> Responder<V> {
    pub(crate) fn new(vault: V, identity_key: Option<Secret>) -> Self {
        Self {
            identity_key,
            signed_prekey: None,
            one_time_prekey: None,
            completed_key_exchange: None,
            state: ResponderState::HandleInitiatorKeys,
            vault,
        }
    }

    fn prologue(&mut self) -> Result<()> {
        let p_atts = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Persistent,
            CURVE25519_SECRET_LENGTH,
        );
        let e_atts = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        if self.identity_key.is_none() {
            self.identity_key = Some(self.vault.secret_generate(p_atts)?);
        }
        self.signed_prekey = Some(self.vault.secret_generate(p_atts)?);
        self.one_time_prekey = Some(self.vault.secret_generate(e_atts)?);
        Ok(())
    }
}

impl<V: X3dhVault> core::fmt::Debug for Responder<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            r#"X3dhResponder {{ identity_key: {:?},
                                      signed_prekey: {:?},
                                      one_time_prekey: {:?},
                                      state: {:?},
                                      vault,
                                      completed_key_exchange: {:?} }}"#,
            self.identity_key,
            self.signed_prekey,
            self.one_time_prekey,
            self.state,
            self.completed_key_exchange
        )
    }
}

#[async_trait]
impl<V: X3dhVault + Sync> KeyExchanger for Responder<V> {
    fn name(&self) -> String {
        "X3DH".to_string()
    }

    fn generate_request(&mut self, _payload: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            ResponderState::SendBundle => {
                let identity_secret_key =
                    self.identity_key.as_ref().ok_or(X3DHError::InvalidState)?;
                let signed_prekey = self.signed_prekey.as_ref().ok_or(X3DHError::InvalidState)?;
                let one_time_prekey = self
                    .one_time_prekey
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;
                let signed_prekey_pub = self.vault.secret_public_key_get(signed_prekey)?;
                let signature = self
                    .vault
                    .sign(identity_secret_key, signed_prekey_pub.as_ref())?;
                let identity_key = self.vault.secret_public_key_get(identity_secret_key)?;
                let one_time_prekey_pub = self.vault.secret_public_key_get(one_time_prekey)?;
                if signature.as_ref().len() != 64 {
                    return Err(X3DHError::SignatureLenMismatch.into());
                }
                let signature_array = array_ref![signature.as_ref(), 0, 64]; //check it against panic
                let bundle = PreKeyBundle {
                    identity_key,
                    signed_prekey: signed_prekey_pub,
                    signature_prekey: Signature(*signature_array),
                    one_time_prekey: one_time_prekey_pub,
                };
                self.state = ResponderState::Done;
                Ok(bundle.to_bytes())
            }
            ResponderState::HandleInitiatorKeys | ResponderState::Done => {
                Err(X3DHError::InvalidState.into())
            }
        }
    }

    async fn async_generate_request(&mut self, _payload: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            ResponderState::SendBundle => {
                let identity_secret_key =
                    self.identity_key.as_ref().ok_or(X3DHError::InvalidState)?;
                let signed_prekey = self.signed_prekey.as_ref().ok_or(X3DHError::InvalidState)?;
                let one_time_prekey = self
                    .one_time_prekey
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;
                let signed_prekey_pub = self.vault.secret_public_key_get(signed_prekey)?;
                let signature = self
                    .vault
                    .sign(identity_secret_key, signed_prekey_pub.as_ref())?;
                let identity_key = self.vault.secret_public_key_get(identity_secret_key)?;
                let one_time_prekey_pub = self.vault.secret_public_key_get(one_time_prekey)?;
                if signature.as_ref().len() != 64 {
                    return Err(X3DHError::SignatureLenMismatch.into());
                }
                let signature_array = array_ref![signature.as_ref(), 0, 64]; //check it against panic
                let bundle = PreKeyBundle {
                    identity_key,
                    signed_prekey: signed_prekey_pub,
                    signature_prekey: Signature(*signature_array),
                    one_time_prekey: one_time_prekey_pub,
                };
                self.state = ResponderState::Done;
                Ok(bundle.to_bytes())
            }
            ResponderState::HandleInitiatorKeys | ResponderState::Done => {
                Err(X3DHError::InvalidState.into())
            }
        }
    }

    fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            ResponderState::HandleInitiatorKeys => {
                if response.len() != 64 {
                    return Err(X3DHError::MessageLenMismatch.into());
                }
                self.prologue()?;

                let other_identity_pubkey = PublicKey::new(array_ref![response, 0, 32].to_vec());
                let other_ephemeral_pubkey = PublicKey::new(array_ref![response, 32, 32].to_vec());

                let signed_prekey = self.signed_prekey.as_ref().ok_or(X3DHError::InvalidState)?;
                let one_time_prekey = self
                    .one_time_prekey
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;

                let local_static_secret =
                    self.identity_key.as_ref().ok_or(X3DHError::InvalidState)?;

                let dh1 = self
                    .vault
                    .ec_diffie_hellman(signed_prekey, &other_identity_pubkey)?;
                let dh2 = self
                    .vault
                    .ec_diffie_hellman(local_static_secret, &other_ephemeral_pubkey)?;
                let dh3 = self
                    .vault
                    .ec_diffie_hellman(signed_prekey, &other_ephemeral_pubkey)?;
                let dh4 = self
                    .vault
                    .ec_diffie_hellman(one_time_prekey, &other_ephemeral_pubkey)?;
                let mut ikm_bytes = vec![0xFFu8; 32]; // FIXME
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh1)?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh2)?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh3)?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh4)?.as_ref());

                let ikm = self.vault.secret_import(
                    &ikm_bytes,
                    SecretAttributes::new(
                        SecretType::Buffer,
                        SecretPersistence::Ephemeral,
                        ikm_bytes.len(),
                    ),
                )?;
                let salt = self.vault.secret_import(
                    &[0u8; 32],
                    SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 32),
                )?;
                let atts = SecretAttributes::new(
                    SecretType::Aes,
                    SecretPersistence::Persistent,
                    AES256_SECRET_LENGTH,
                );

                let mut keyrefs =
                    self.vault
                        .hkdf_sha256(&salt, CSUITE, Some(&ikm), vec![atts, atts])?;
                let decrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;
                let encrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;
                let mut state_hash = self.vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut ikm_bytes);
                let state_hash = self.vault.sha256(state_hash.as_slice())?;

                self.completed_key_exchange = Some(CompletedKeyExchange::new(
                    state_hash,
                    encrypt_key,
                    decrypt_key,
                ));
                self.state = ResponderState::SendBundle;
                Ok(vec![])
            }
            ResponderState::SendBundle | ResponderState::Done => {
                Err(X3DHError::InvalidState.into())
            }
        }
    }

    async fn async_handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        self.handle_response(response)
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(self) -> Result<CompletedKeyExchange> {
        self.completed_key_exchange
            .ok_or_else(|| X3DHError::InvalidState.into())
    }

    async fn async_finalize(self) -> Result<CompletedKeyExchange> {
        self.finalize()
    }
}
