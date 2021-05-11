use crate::{PreKeyBundle, Signature, X3DHError, X3dhVault, CSUITE, ENROLLMENT_MSG_SIZE};
use arrayref::array_ref;
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_SECRET_LENGTH,
};
use subtle::ConstantTimeEq;

#[derive(Debug)]
enum ResponderState {
    /// Create a PreKey Bundle
    GenerateBundle,
    /// Expect an enrollment message from this EIK
    SetEnrollmentKey,
    /// Verify an enrollment message
    VerifyEnrollment,
    /// Done
    Done,
}

/// The responder of X3DH creates a prekey bundle that can be used to establish a shared
/// secret key with another party that can use
pub struct Responder<V: X3dhVault> {
    // Identity key and signer prekey are wrapped in Arc because they are possible shared
    // among threads/modules
    identity_key: Option<Secret>,
    signed_prekey: Option<Secret>,
    one_time_prekey: Option<Secret>,
    expected_enrollment_key: Option<PublicKey>,
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
            expected_enrollment_key: None,
            completed_key_exchange: None,
            state: ResponderState::GenerateBundle,
            vault,
        }
    }

    fn prologue(&mut self) -> ockam_core::Result<()> {
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
        self.expected_enrollment_key = None;
        self.completed_key_exchange = None;
        Ok(())
    }
}

impl<V: X3dhVault> std::fmt::Debug for Responder<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            r#"X3dhResponder {{ identity_key: {:?},
                                      signed_prekey: {:?},
                                      one_time_prekey: {:?},
                                      expected_enrollment_key: {:?},
                                      state: {:?},
                                      vault,
                                      completed_key_exchange: {:?} }}"#,
            self.identity_key,
            self.signed_prekey,
            self.one_time_prekey,
            self.expected_enrollment_key,
            self.state,
            self.completed_key_exchange
        )
    }
}

impl<V: X3dhVault> KeyExchanger for Responder<V> {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
        match self.state {
            ResponderState::GenerateBundle => {
                self.prologue()?;
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
                let bundle = PreKeyBundle {
                    identity_key,
                    signed_prekey: signed_prekey_pub,
                    signature_prekey: Signature(signature),
                    one_time_prekey: one_time_prekey_pub,
                };
                self.state = ResponderState::SetEnrollmentKey;
                Ok(bundle.to_bytes())
            }
            ResponderState::SetEnrollmentKey => {
                if data.len() != 32 {
                    return Err(X3DHError::MessageLenMismatch.into());
                }
                self.expected_enrollment_key =
                    Some(PublicKey::new(array_ref![data, 0, 32].to_vec()));
                self.state = ResponderState::VerifyEnrollment;
                Ok(vec![])
            }
            ResponderState::VerifyEnrollment => {
                if data.len() != ENROLLMENT_MSG_SIZE {
                    return Err(X3DHError::MessageLenMismatch.into());
                }
                let signed_prekey = self.signed_prekey.as_ref().ok_or(X3DHError::InvalidState)?;
                let one_time_prekey = self
                    .one_time_prekey
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;
                let eik = self
                    .expected_enrollment_key
                    .as_ref()
                    .ok_or(X3DHError::InvalidState)?;
                let id = self.vault.sha256(eik.as_ref())?;
                if id.ct_eq(&data[32..64]).unwrap_u8() != 1 {
                    return Err(X3DHError::InvalidHash.into());
                }
                let ek = PublicKey::new(array_ref![data, 0, 32].to_vec());
                let local_static_secret =
                    self.identity_key.take().ok_or(X3DHError::InvalidState)?;

                let dh1 = self.vault.ec_diffie_hellman(signed_prekey, &eik)?;
                let dh2 = self.vault.ec_diffie_hellman(&local_static_secret, &ek)?;
                let dh3 = self.vault.ec_diffie_hellman(signed_prekey, &ek)?;
                let dh4 = self.vault.ec_diffie_hellman(one_time_prekey, &ek)?;
                let mut ikm_bytes = vec![0xFFu8; 32];
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

                let mut aad = data[..64].to_vec();
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);
                //TODO: get the channel address from the message somehow if needed
                let plaintext = self.vault.aead_aes_gcm_decrypt(
                    &decrypt_key,
                    &data[64..],
                    &data[..12],
                    aad.as_slice(),
                )?;
                let ikb = PublicKey::new(array_ref![plaintext, 0, 32].to_vec());
                let signature = array_ref![plaintext, 32, 64];
                self.vault.verify(signature, &eik, &plaintext[..32])?;

                self.completed_key_exchange = Some(CompletedKeyExchange::new(
                    state_hash,
                    encrypt_key,
                    decrypt_key,
                    local_static_secret,
                    ikb,
                ));
                self.state = ResponderState::Done;
                Ok(vec![])
            }
            ResponderState::Done => Err(X3DHError::InvalidState.into()),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        self.completed_key_exchange
            .ok_or_else(|| X3DHError::InvalidState.into())
    }
}
