use crate::{PreKeyBundle, X3DHError, X3dhVault, CSUITE};
use ockam_core::lib::convert::TryFrom;
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};
use ockam_vault_core::{
    Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_SECRET_LENGTH,
};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
enum InitiatorState {
    GenerateEphemeralIdentityKey,
    ProcessPreKeyBundle,
    Done,
}

/// The responder of X3DH receives a prekey bundle and computes the shared secret
/// to communicate the first message to the initiator
pub struct Initiator {
    ephemeral_identity_key: Option<Secret>,
    prekey_bundle: Option<PreKeyBundle>,
    state: InitiatorState,
    vault: Arc<Mutex<dyn X3dhVault>>,
    completed_key_exchange: Option<CompletedKeyExchange>,
    identity_key: Option<Secret>,
}

impl Initiator {
    pub(crate) fn new(v: Arc<Mutex<dyn X3dhVault>>, identity_key: Option<Secret>) -> Self {
        Self {
            ephemeral_identity_key: None,
            prekey_bundle: None,
            state: InitiatorState::GenerateEphemeralIdentityKey,
            vault: v,
            completed_key_exchange: None,
            identity_key,
        }
    }

    fn prologue(&mut self) -> ockam_core::Result<()> {
        let mut vault = self.vault.lock().unwrap();
        let p_atts = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Persistent,
            CURVE25519_SECRET_LENGTH,
        );
        self.ephemeral_identity_key = Some(vault.secret_generate(p_atts)?);
        Ok(())
    }
}

impl std::fmt::Debug for Initiator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

impl KeyExchanger for Initiator {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
        match self.state {
            InitiatorState::GenerateEphemeralIdentityKey => {
                self.prologue()?;
                let mut vault = self.vault.lock().unwrap();
                let ephemeral_identity_key = vault.secret_generate(SecretAttributes::new(
                    SecretType::Curve25519,
                    SecretPersistence::Persistent,
                    CURVE25519_SECRET_LENGTH,
                ))?;
                let pubkey = vault.secret_public_key_get(&ephemeral_identity_key)?;
                self.ephemeral_identity_key = Some(ephemeral_identity_key);
                self.state = InitiatorState::ProcessPreKeyBundle;
                Ok(pubkey.as_ref().to_vec())
            }
            InitiatorState::ProcessPreKeyBundle => {
                let prekey_bundle = PreKeyBundle::try_from(data)?;

                let mut vault = self.vault.lock().unwrap();

                let ephemeral_identity_key = self
                    .ephemeral_identity_key
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;

                // Check the prekey_bundle signature
                vault.verify(
                    prekey_bundle.signature_prekey.as_ref(),
                    &prekey_bundle.identity_key,
                    prekey_bundle.signed_prekey.as_ref(),
                )?;
                let atts = SecretAttributes::new(
                    SecretType::Curve25519,
                    SecretPersistence::Ephemeral,
                    CURVE25519_SECRET_LENGTH,
                );
                let esk = vault.secret_generate(atts)?;
                let dh1 = vault
                    .ec_diffie_hellman(ephemeral_identity_key, &prekey_bundle.signed_prekey)?;
                let dh2 = vault.ec_diffie_hellman(&esk, &prekey_bundle.identity_key)?;
                let dh3 = vault.ec_diffie_hellman(&esk, &prekey_bundle.signed_prekey)?;
                let dh4 = vault.ec_diffie_hellman(&esk, &prekey_bundle.one_time_prekey)?;
                let mut ikm_bytes = vec![0xFFu8; 32];
                ikm_bytes.extend_from_slice(vault.secret_export(&dh1)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh2)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh3)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh4)?.as_ref());

                let ikm = vault.secret_import(
                    &ikm_bytes,
                    SecretAttributes::new(
                        SecretType::Buffer,
                        SecretPersistence::Ephemeral,
                        ikm_bytes.len(),
                    ),
                )?;
                let salt = vault.secret_import(
                    &[0u8; 32],
                    SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 32),
                )?;

                let atts = SecretAttributes::new(
                    SecretType::Aes,
                    SecretPersistence::Persistent,
                    AES256_SECRET_LENGTH,
                );

                let mut keyrefs = vault.hkdf_sha256(&salt, CSUITE, Some(&ikm), vec![atts, atts])?;
                let encrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;
                let decrypt_key = keyrefs.pop().ok_or(X3DHError::InvalidState)?;
                let ek = vault.secret_public_key_get(&esk)?;
                let pubkey = vault.secret_public_key_get(ephemeral_identity_key)?;

                let mut state_hash = vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut ikm_bytes);
                let state_hash = vault.sha256(state_hash.as_slice())?;

                let mut aad = ek.as_ref().to_vec();
                aad.extend_from_slice(&vault.sha256(pubkey.as_ref())?);
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);

                let atts = SecretAttributes::new(
                    SecretType::Curve25519,
                    SecretPersistence::Persistent,
                    CURVE25519_SECRET_LENGTH,
                );

                let skb = if let Some(ik) = &self.identity_key {
                    ik.clone()
                } else {
                    vault.secret_generate(atts)?
                };
                let ikb = vault.secret_public_key_get(&skb)?;

                let mut plaintext = ikb.as_ref().to_vec();
                plaintext.extend_from_slice(&vault.sign(ephemeral_identity_key, ikb.as_ref())?);

                let mut ciphertext_and_tag = vault.aead_aes_gcm_encrypt(
                    &encrypt_key,
                    plaintext.as_slice(),
                    &ek.as_ref()[..12],
                    aad.as_slice(),
                )?;
                let mut output = aad[..64].to_vec();
                output.append(&mut ciphertext_and_tag);
                self.completed_key_exchange = Some(CompletedKeyExchange::new(
                    state_hash,
                    encrypt_key,
                    decrypt_key,
                    skb,
                    prekey_bundle.identity_key,
                ));
                self.state = InitiatorState::Done;
                Ok(output)
            }
            InitiatorState::Done => Err(X3DHError::InvalidState.into()),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, InitiatorState::Done)
    }

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        self.completed_key_exchange
            .ok_or_else(|| X3DHError::InvalidState.into())
    }

    fn finalize_box(self: Box<Self>) -> ockam_core::Result<CompletedKeyExchange> {
        self.finalize()
    }
}
