use crate::{PreKeyBundle, X3DHError, X3dhVault, CSUITE};
use alloc::vec;
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::vault::Signature as GenericSignature;
use ockam_core::vault::{
    KeyId, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_SECRET_LENGTH,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};

#[derive(Debug, Clone, Copy)]
enum InitiatorState {
    GenerateEphemeralIdentityKey,
    ProcessPreKeyBundle,
    Done,
}

/// The initiator of X3DH receives a prekey bundle and computes the shared secret
/// to communicate the first message to the responder
pub struct Initiator<V: X3dhVault> {
    identity_key: Option<KeyId>,
    ephemeral_identity_key: Option<KeyId>,
    prekey_bundle: Option<PreKeyBundle>,
    state: InitiatorState,
    vault: V,
    completed_key_exchange: Option<CompletedKeyExchange>,
}

impl<V: X3dhVault> Initiator<V> {
    pub(crate) fn new(vault: V, identity_key: Option<KeyId>) -> Self {
        Self {
            identity_key,
            ephemeral_identity_key: None,
            prekey_bundle: None,
            state: InitiatorState::GenerateEphemeralIdentityKey,
            vault,
            completed_key_exchange: None,
        }
    }

    async fn prologue(&mut self) -> Result<()> {
        if self.identity_key.is_none() {
            let p_atts = SecretAttributes::new(
                SecretType::X25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            );

            self.identity_key = Some(self.vault.secret_generate(p_atts).await?);
        }
        Ok(())
    }
}

impl<V: X3dhVault> core::fmt::Debug for Initiator<V> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            r#"X3dhInitiator {{ ephemeral_identity_key: {:?}, prekey_bundle: {:?}, state: {:?}, vault, completed_key_exchange: {:?}, identity_key: {:?} }}"#,
            self.ephemeral_identity_key,
            self.prekey_bundle,
            self.state,
            self.completed_key_exchange,
            self.identity_key,
        )
    }
}

#[async_trait]
impl<V: X3dhVault> KeyExchanger for Initiator<V> {
    async fn name(&self) -> Result<String> {
        Ok("X3DH".to_string())
    }

    async fn generate_request(&mut self, _payload: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            InitiatorState::GenerateEphemeralIdentityKey => {
                self.prologue().await?;
                let identity_key = self.identity_key.as_ref().ok_or(X3DHError::InvalidState)?;
                let pubkey = self.vault.secret_public_key_get(identity_key).await?;

                let ephemeral_identity_key = self
                    .vault
                    .secret_generate(SecretAttributes::new(
                        SecretType::X25519,
                        SecretPersistence::Ephemeral,
                        CURVE25519_SECRET_LENGTH,
                    ))
                    .await?;
                let ephemeral_pubkey = self
                    .vault
                    .secret_public_key_get(&ephemeral_identity_key)
                    .await?;
                self.ephemeral_identity_key = Some(ephemeral_identity_key);
                self.state = InitiatorState::ProcessPreKeyBundle;

                let mut response = Vec::new();
                response.extend_from_slice(pubkey.data());
                response.extend_from_slice(ephemeral_pubkey.data());
                Ok(response)
            }
            InitiatorState::ProcessPreKeyBundle | InitiatorState::Done => {
                Err(X3DHError::InvalidState.into())
            }
        }
    }

    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        match self.state {
            InitiatorState::ProcessPreKeyBundle => {
                let prekey_bundle = PreKeyBundle::try_from(response)?;

                let identity_key = self.identity_key.as_ref().ok_or(X3DHError::InvalidState)?;

                let ephemeral_identity_key = self
                    .ephemeral_identity_key
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;

                // Check the prekey_bundle signature
                self.vault
                    .verify(
                        &GenericSignature::new(prekey_bundle.signature_prekey.as_ref().to_vec()),
                        &prekey_bundle.identity_key,
                        prekey_bundle.signed_prekey.data(),
                    )
                    .await?;

                let dh1 = self
                    .vault
                    .ec_diffie_hellman(identity_key, &prekey_bundle.signed_prekey)
                    .await?;
                let dh2 = self
                    .vault
                    .ec_diffie_hellman(ephemeral_identity_key, &prekey_bundle.identity_key)
                    .await?;
                let dh3 = self
                    .vault
                    .ec_diffie_hellman(ephemeral_identity_key, &prekey_bundle.signed_prekey)
                    .await?;
                let dh4 = self
                    .vault
                    .ec_diffie_hellman(ephemeral_identity_key, &prekey_bundle.one_time_prekey)
                    .await?;
                let mut ikm_bytes = vec![0xFFu8; 32]; // FIXME: Why is it here?
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh1).await?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh2).await?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh3).await?.as_ref());
                ikm_bytes.extend_from_slice(self.vault.secret_export(&dh4).await?.as_ref());

                let ikm = self
                    .vault
                    .secret_import(
                        &ikm_bytes,
                        SecretAttributes::new(
                            SecretType::Buffer,
                            SecretPersistence::Ephemeral,
                            ikm_bytes.len(),
                        ),
                    )
                    .await?;
                let salt = self
                    .vault
                    .secret_import(
                        &[0u8; 32],
                        SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 32),
                    )
                    .await?;

                let atts = SecretAttributes::new(
                    SecretType::Aes,
                    SecretPersistence::Persistent,
                    AES256_SECRET_LENGTH,
                );

                let mut keyrefs = self
                    .vault
                    .hkdf_sha256(&salt, CSUITE, Some(&ikm), vec![atts, atts])
                    .await?;
                let encrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;
                let decrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;

                let mut state_hash = self.vault.sha256(CSUITE).await?.to_vec();
                state_hash.append(&mut ikm_bytes);
                let state_hash = self.vault.sha256(state_hash.as_slice()).await?;

                self.completed_key_exchange = Some(CompletedKeyExchange::new(
                    state_hash,
                    encrypt_key,
                    decrypt_key,
                ));
                self.state = InitiatorState::Done;
                Ok(vec![])
            }
            InitiatorState::GenerateEphemeralIdentityKey | InitiatorState::Done => {
                Err(X3DHError::InvalidState.into())
            }
        }
    }

    async fn is_complete(&self) -> Result<bool> {
        Ok(matches!(self.state, InitiatorState::Done))
    }

    async fn finalize(self) -> Result<CompletedKeyExchange> {
        self.completed_key_exchange
            .ok_or_else(|| X3DHError::InvalidState.into())
    }
}
