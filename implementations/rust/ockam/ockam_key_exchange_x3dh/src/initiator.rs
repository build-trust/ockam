use crate::{PreKeyBundle, X3DHError, X3dhVault, CSUITE};
use alloc::vec;
use ockam_core::vault::Signature as GenericSignature;
use ockam_core::vault::{
    KeyAttributes, KeyId, KeyPersistence, KeyType, AES256_SECRET_LENGTH_U32,
    CURVE25519_SECRET_LENGTH_U32,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{
    compat::{
        string::{String, ToString},
        vec::Vec,
    },
    vault::{Key, PrivateKey},
};
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
            let p_atts = KeyAttributes::new(
                KeyType::X25519,
                KeyPersistence::Persistent,
                CURVE25519_SECRET_LENGTH_U32,
            );

            self.identity_key = Some(self.vault.generate_key(p_atts).await?);
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
                let pubkey = self.vault.get_public_key(identity_key).await?;

                let ephemeral_identity_key = self
                    .vault
                    .generate_key(KeyAttributes::new(
                        KeyType::X25519,
                        KeyPersistence::Ephemeral,
                        CURVE25519_SECRET_LENGTH_U32,
                    ))
                    .await?;
                let ephemeral_pubkey = self.vault.get_public_key(&ephemeral_identity_key).await?;
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
                ikm_bytes
                    .extend_from_slice(self.vault.export_key(&dh1).await?.try_as_key()?.as_ref());
                ikm_bytes
                    .extend_from_slice(self.vault.export_key(&dh2).await?.try_as_key()?.as_ref());
                ikm_bytes
                    .extend_from_slice(self.vault.export_key(&dh3).await?.try_as_key()?.as_ref());
                ikm_bytes
                    .extend_from_slice(self.vault.export_key(&dh4).await?.try_as_key()?.as_ref());

                let ikm = self
                    .vault
                    .import_key(
                        Key::Key(PrivateKey::new(ikm_bytes.clone())),
                        KeyAttributes::new(
                            KeyType::Buffer,
                            KeyPersistence::Ephemeral,
                            ikm_bytes.len() as u32,
                        ),
                    )
                    .await?;
                let salt = self
                    .vault
                    .import_key(
                        Key::Key(PrivateKey::new(vec![0u8; 32])),
                        KeyAttributes::new(KeyType::Buffer, KeyPersistence::Ephemeral, 32u32),
                    )
                    .await?;

                let atts = KeyAttributes::new(
                    KeyType::Aes,
                    KeyPersistence::Persistent,
                    AES256_SECRET_LENGTH_U32,
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
