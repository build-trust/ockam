use crate::error::{KexExchangeFailError, KeyExchangeFailErrorKind};
use crate::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_vault::types::{
    SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH, CURVE25519_SECRET_LENGTH,
};
use ockam_vault::{
    error::VaultFailError, types::PublicKey, AsymmetricVault, HashVault, Secret, SecretVault,
    SignerVault, SymmetricVault, VerifierVault,
};
use std::{
    convert::TryFrom,
    sync::{Arc, Mutex},
};
use subtle::ConstantTimeEq;

/// Represents and (X)EdDSA or ECDSA signature
/// from Ed25519 or P-256
#[derive(Clone, Copy)]
pub struct Signature([u8; 64]);

impl AsRef<[u8; 64]> for Signature {
    fn as_ref(&self) -> &[u8; 64] {
        &self.0
    }
}

impl From<[u8; 64]> for Signature {
    fn from(data: [u8; 64]) -> Self {
        Signature(data)
    }
}

impl From<&[u8; 64]> for Signature {
    fn from(data: &[u8; 64]) -> Self {
        Signature(*data)
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Signature {{ {} }}", hex::encode(self.0.as_ref()))
    }
}

/// Represents all the keys and signature to send to an enrollee
#[derive(Clone, Debug)]
pub struct PreKeyBundle {
    identity_key: PublicKey,
    signed_prekey: PublicKey,
    signature_prekey: Signature,
    one_time_prekey: PublicKey,
}

impl PreKeyBundle {
    const SIZE: usize = 32 + 32 + 64 + 32;
    /// Convert the prekey bundle to a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(self.identity_key.as_ref());
        output.extend_from_slice(self.signed_prekey.as_ref());
        output.extend_from_slice(self.signature_prekey.0.as_ref());
        output.extend_from_slice(self.one_time_prekey.as_ref());
        output
    }
}

impl TryFrom<&[u8]> for PreKeyBundle {
    type Error = KexExchangeFailError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::SIZE {
            return Err(KeyExchangeFailErrorKind::InvalidByteCount(Self::SIZE, data.len()).into());
        }
        let identity_key = PublicKey::new(array_ref![data, 0, 32].to_vec());
        let signed_prekey = PublicKey::new(array_ref![data, 32, 32].to_vec());
        let signature_prekey = Signature(*array_ref![data, 64, 64]);
        let one_time_prekey = PublicKey::new(array_ref![data, 128, 32].to_vec());
        Ok(Self {
            identity_key,
            signed_prekey,
            signature_prekey,
            one_time_prekey,
        })
    }
}

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

const CSUITE: &[u8] = b"X3DH_25519_AESGCM_SHA256\0\0\0\0\0\0\0\0";
/// EK, Hash(EIK), IK, EdDSA, AES_GCM_TAG
const ENROLLMENT_MSG_SIZE: usize = 32 + 32 + 32 + 64 + 16;

/// Vault with X3DH required functionality
pub trait X3dhVault:
    SecretVault + SignerVault + VerifierVault + AsymmetricVault + SymmetricVault + HashVault + Send
{
}

impl<D> X3dhVault for D where
    D: SecretVault
        + SignerVault
        + VerifierVault
        + AsymmetricVault
        + SymmetricVault
        + HashVault
        + Send
{
}

/// The responder of X3DH creates a prekey bundle that can be used to establish a shared
/// secret key with another party that can use
pub struct X3dhResponder {
    // Identity key and signer prekey are wrapped in Arc because they are possible shared
    // among threads/modules
    identity_key: Option<Arc<Box<dyn Secret>>>,
    signed_prekey: Option<Arc<Box<dyn Secret>>>,
    one_time_prekey: Option<Box<dyn Secret>>,
    expected_enrollment_key: Option<PublicKey>,
    state: ResponderState,
    vault: Arc<Mutex<dyn X3dhVault>>,
    completed_key_exchange: Option<CompletedKeyExchange>,
}

impl X3dhResponder {
    fn new(v: Arc<Mutex<dyn X3dhVault>>, identity_key: Option<Arc<Box<dyn Secret>>>) -> Self {
        Self {
            identity_key,
            signed_prekey: None,
            one_time_prekey: None,
            expected_enrollment_key: None,
            completed_key_exchange: None,
            state: ResponderState::GenerateBundle,
            vault: v,
        }
    }

    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let mut vault = self.vault.lock().unwrap();
        let p_atts = SecretAttributes {
            persistence: SecretPersistence::Persistent,
            stype: SecretType::Curve25519,
            length: CURVE25519_SECRET_LENGTH,
        };
        let e_atts = SecretAttributes {
            persistence: SecretPersistence::Ephemeral,
            stype: SecretType::Curve25519,
            length: CURVE25519_SECRET_LENGTH,
        };
        if self.identity_key.is_none() {
            self.identity_key = Some(Arc::new(vault.secret_generate(p_atts)?));
        }
        self.signed_prekey = Some(Arc::new(vault.secret_generate(p_atts)?));
        self.one_time_prekey = Some(vault.secret_generate(e_atts)?);
        self.expected_enrollment_key = None;
        self.completed_key_exchange = None;
        Ok(())
    }
}

impl std::fmt::Debug for X3dhResponder {
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

#[derive(Debug, Clone, Copy)]
enum InitiatorState {
    GenerateEphemeralIdentityKey,
    ProcessPreKeyBundle,
    Done,
}

/// The responder of X3DH receives a prekey bundle and computes the shared secret
/// to communicate the first message to the initiator
pub struct X3dhInitiator {
    ephemeral_identity_key: Option<Box<dyn Secret>>,
    prekey_bundle: Option<PreKeyBundle>,
    state: InitiatorState,
    vault: Arc<Mutex<dyn X3dhVault>>,
    completed_key_exchange: Option<CompletedKeyExchange>,
    identity_key: Option<Arc<Box<dyn Secret>>>,
}

impl X3dhInitiator {
    fn new(v: Arc<Mutex<dyn X3dhVault>>, identity_key: Option<Arc<Box<dyn Secret>>>) -> Self {
        Self {
            ephemeral_identity_key: None,
            prekey_bundle: None,
            state: InitiatorState::GenerateEphemeralIdentityKey,
            vault: v,
            completed_key_exchange: None,
            identity_key,
        }
    }

    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let mut vault = self.vault.lock().unwrap();
        let p_atts = SecretAttributes {
            persistence: SecretPersistence::Persistent,
            stype: SecretType::Curve25519,
            length: CURVE25519_SECRET_LENGTH,
        };
        self.ephemeral_identity_key = Some(vault.secret_generate(p_atts)?);
        self.prekey_bundle = None;
        self.completed_key_exchange = None;
        Ok(())
    }
}

impl std::fmt::Debug for X3dhInitiator {
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

impl KeyExchanger for X3dhResponder {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            ResponderState::GenerateBundle => {
                self.prologue()?;
                let mut vault = self.vault.lock().unwrap();
                let identity_secret_key =
                    self.identity_key
                        .as_ref()
                        .ok_or(KeyExchangeFailErrorKind::GeneralError {
                            msg: "Invalid identity key".to_string(),
                        })?;
                let signed_prekey =
                    self.signed_prekey
                        .as_ref()
                        .ok_or(KeyExchangeFailErrorKind::GeneralError {
                            msg: "Invalid signer prekey".to_string(),
                        })?;
                let one_time_prekey = self.one_time_prekey.as_ref().ok_or(
                    KeyExchangeFailErrorKind::GeneralError {
                        msg: "Invalid one-time prekey".to_string(),
                    },
                )?;
                let signed_prekey_pub = vault.secret_public_key_get(signed_prekey)?;
                let signature = vault.sign(identity_secret_key, signed_prekey_pub.as_ref())?;
                let identity_key = vault.secret_public_key_get(identity_secret_key)?;
                let one_time_prekey_pub = vault.secret_public_key_get(one_time_prekey)?;
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
                    return Err(KeyExchangeFailErrorKind::InvalidByteCount(32, data.len()).into());
                }
                self.expected_enrollment_key =
                    Some(PublicKey::new(array_ref![data, 0, 32].to_vec()));
                self.state = ResponderState::VerifyEnrollment;
                Ok(vec![])
            }
            ResponderState::VerifyEnrollment => {
                debug_assert!(self.expected_enrollment_key.is_some());
                if data.len() != ENROLLMENT_MSG_SIZE {
                    return Err(KeyExchangeFailErrorKind::InvalidByteCount(
                        ENROLLMENT_MSG_SIZE,
                        data.len(),
                    )
                    .into());
                }
                let mut vault = self.vault.lock().unwrap();
                let signed_prekey =
                    self.signed_prekey
                        .as_ref()
                        .ok_or(KeyExchangeFailErrorKind::GeneralError {
                            msg: "Invalid signer prekey".to_string(),
                        })?;
                let one_time_prekey = self.one_time_prekey.as_ref().ok_or(
                    KeyExchangeFailErrorKind::GeneralError {
                        msg: "Invalid one-time prekey".to_string(),
                    },
                )?;
                let eik = self.expected_enrollment_key.as_ref().unwrap();
                let id = vault.sha256(eik.as_ref())?;
                if id.ct_eq(&data[32..64]).unwrap_u8() != 1 {
                    return Err(KeyExchangeFailErrorKind::InvalidHash {
                        expected: hex::encode(id),
                        actual: hex::encode(&data[32..64]),
                    }
                    .into());
                }
                let ek = PublicKey::new(array_ref![data, 0, 32].to_vec());
                let local_static_secret =
                    self.identity_key
                        .take()
                        .ok_or(KeyExchangeFailErrorKind::GeneralError {
                            msg: "Invalid identity key".to_string(),
                        })?;

                let dh1 = vault.ec_diffie_hellman(signed_prekey, eik.as_ref())?;
                let dh2 = vault.ec_diffie_hellman(&local_static_secret, ek.as_ref())?;
                let dh3 = vault.ec_diffie_hellman(signed_prekey, ek.as_ref())?;
                let dh4 = vault.ec_diffie_hellman(one_time_prekey, ek.as_ref())?;
                let mut ikm_bytes = vec![0xFFu8; 32];
                ikm_bytes.extend_from_slice(vault.secret_export(&dh1)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh2)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh3)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh4)?.as_ref());

                let ikm = vault.secret_import(
                    &ikm_bytes,
                    SecretAttributes {
                        persistence: SecretPersistence::Ephemeral,
                        stype: SecretType::Buffer,
                        length: ikm_bytes.len(),
                    },
                )?;
                let salt = vault.secret_import(
                    &[0u8; 32],
                    SecretAttributes {
                        persistence: SecretPersistence::Ephemeral,
                        stype: SecretType::Buffer,
                        length: 32,
                    },
                )?;
                let atts = SecretAttributes {
                    persistence: SecretPersistence::Persistent,
                    stype: SecretType::Aes,
                    length: AES256_SECRET_LENGTH,
                };

                let mut keyrefs = vault.hkdf_sha256(&salt, CSUITE, Some(&ikm), vec![atts, atts])?;
                let decrypt_key = keyrefs.pop().unwrap();
                let encrypt_key = keyrefs.pop().unwrap();
                let mut state_hash = vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut ikm_bytes);
                let state_hash = vault.sha256(state_hash.as_slice())?;

                let mut aad = data[..64].to_vec();
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);
                //TODO: get the channel address from the message somehow if needed
                let plaintext = vault.aead_aes_gcm_decrypt(
                    &decrypt_key,
                    &data[64..],
                    &data[..12],
                    aad.as_slice(),
                )?;
                let ikb = PublicKey::new(array_ref![plaintext, 0, 32].to_vec());
                let signature = array_ref![plaintext, 32, 64];
                vault.verify(signature, eik.as_ref(), &plaintext[..32])?;

                self.completed_key_exchange = Some(CompletedKeyExchange {
                    h: state_hash,
                    encrypt_key,
                    decrypt_key,
                    local_static_secret,
                    remote_static_public_key: ikb,
                });
                self.state = ResponderState::Done;
                Ok(vec![])
            }
            ResponderState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(self: Box<Self>) -> Result<CompletedKeyExchange, VaultFailError> {
        Ok(self.completed_key_exchange.unwrap())
    }
}

impl KeyExchanger for X3dhInitiator {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            InitiatorState::GenerateEphemeralIdentityKey => {
                self.prologue()?;
                let mut vault = self.vault.lock().unwrap();
                let ephemeral_identity_key = vault.secret_generate(SecretAttributes {
                    persistence: SecretPersistence::Persistent,
                    stype: SecretType::Curve25519,
                    length: CURVE25519_SECRET_LENGTH,
                })?;
                let pubkey = vault.secret_public_key_get(&ephemeral_identity_key)?;
                self.ephemeral_identity_key = Some(ephemeral_identity_key);
                self.state = InitiatorState::ProcessPreKeyBundle;
                Ok(pubkey.as_ref().to_vec())
            }
            InitiatorState::ProcessPreKeyBundle => {
                let prekey_bundle = PreKeyBundle::try_from(data)?;

                let mut vault = self.vault.lock().unwrap();

                let ephemeral_identity_key = self.ephemeral_identity_key.as_ref().ok_or(
                    KeyExchangeFailErrorKind::GeneralError {
                        msg: "Invalid ephemeral identity key".to_string(),
                    },
                )?;

                // Check the prekey_bundle signature
                vault.verify(
                    prekey_bundle.signature_prekey.as_ref(),
                    prekey_bundle.identity_key.as_ref(),
                    prekey_bundle.signed_prekey.as_ref(),
                )?;
                let atts = SecretAttributes {
                    persistence: SecretPersistence::Ephemeral,
                    stype: SecretType::Curve25519,
                    length: CURVE25519_SECRET_LENGTH,
                };
                let esk = vault.secret_generate(atts)?;
                let dh1 = vault.ec_diffie_hellman(
                    ephemeral_identity_key,
                    prekey_bundle.signed_prekey.as_ref(),
                )?;
                let dh2 = vault.ec_diffie_hellman(&esk, prekey_bundle.identity_key.as_ref())?;
                let dh3 = vault.ec_diffie_hellman(&esk, prekey_bundle.signed_prekey.as_ref())?;
                let dh4 = vault.ec_diffie_hellman(&esk, prekey_bundle.one_time_prekey.as_ref())?;
                let mut ikm_bytes = vec![0xFFu8; 32];
                ikm_bytes.extend_from_slice(vault.secret_export(&dh1)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh2)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh3)?.as_ref());
                ikm_bytes.extend_from_slice(vault.secret_export(&dh4)?.as_ref());

                let ikm = vault.secret_import(
                    &ikm_bytes,
                    SecretAttributes {
                        persistence: SecretPersistence::Ephemeral,
                        stype: SecretType::Buffer,
                        length: ikm_bytes.len(),
                    },
                )?;
                let salt = vault.secret_import(
                    &[0u8; 32],
                    SecretAttributes {
                        persistence: SecretPersistence::Ephemeral,
                        stype: SecretType::Buffer,
                        length: 32,
                    },
                )?;

                let mut atts = SecretAttributes {
                    stype: SecretType::Aes,
                    persistence: SecretPersistence::Persistent,
                    length: AES256_SECRET_LENGTH,
                };

                let mut keyrefs = vault.hkdf_sha256(&salt, CSUITE, Some(&ikm), vec![atts, atts])?;
                let encrypt_key = keyrefs.pop().unwrap();
                let decrypt_key = keyrefs.pop().unwrap();
                let ek = vault.secret_public_key_get(&esk)?;
                let pubkey = vault.secret_public_key_get(ephemeral_identity_key)?;

                let mut state_hash = vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut ikm_bytes);
                let state_hash = vault.sha256(state_hash.as_slice())?;

                let mut aad = ek.as_ref().to_vec();
                aad.extend_from_slice(&vault.sha256(pubkey.as_ref())?);
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);

                atts.stype = SecretType::Curve25519;
                let identity_key = self.identity_key.take();
                let skb = if identity_key.is_none() {
                    Arc::new(vault.secret_generate(atts)?)
                } else {
                    identity_key.unwrap()
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
                self.completed_key_exchange = Some(CompletedKeyExchange {
                    h: state_hash,
                    encrypt_key,
                    decrypt_key,
                    local_static_secret: skb,
                    remote_static_public_key: prekey_bundle.identity_key,
                });
                self.state = InitiatorState::Done;
                Ok(output)
            }
            InitiatorState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, InitiatorState::Done)
    }

    fn finalize(self: Box<Self>) -> Result<CompletedKeyExchange, VaultFailError> {
        Ok(self.completed_key_exchange.unwrap())
    }
}

/// Represents an XX NewKeyExchanger
pub struct X3dhNewKeyExchanger {
    vault_initiator: Arc<Mutex<dyn X3dhVault>>,
    vault_responder: Arc<Mutex<dyn X3dhVault>>,
}

impl std::fmt::Debug for X3dhNewKeyExchanger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "X3dhNewKeyExchanger {{ vault_initiator, vault_responder }}"
        )
    }
}

impl X3dhNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(
        vault_initiator: Arc<Mutex<dyn X3dhVault>>,
        vault_responder: Arc<Mutex<dyn X3dhVault>>,
    ) -> Self {
        Self {
            vault_initiator,
            vault_responder,
        }
    }
}

impl NewKeyExchanger<X3dhInitiator, X3dhResponder> for X3dhNewKeyExchanger {
    fn initiator(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> X3dhInitiator {
        X3dhInitiator::new(self.vault_initiator.clone(), identity_key)
    }

    fn responder(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> X3dhResponder {
        X3dhResponder::new(self.vault_responder.clone(), identity_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_vault_software::DefaultVault;

    #[test]
    fn handshake() {
        let vault_i = Arc::new(Mutex::new(DefaultVault::default()));
        let vault_r = Arc::new(Mutex::new(DefaultVault::default()));
        let mut initiator = X3dhInitiator::new(vault_i.clone(), None);
        let mut responder = X3dhResponder::new(vault_r.clone(), None);

        assert!(initiator.prologue().is_ok());
        assert!(responder.prologue().is_ok());

        let res = initiator.process(&[]);
        assert!(res.is_ok());
        let eik_bytes = res.unwrap();
        assert_eq!(eik_bytes.len(), 32);
        let res = responder.process(&[]);
        assert!(res.is_ok());
        let prekey_bundle_bytes = res.unwrap();

        let res = initiator.process(prekey_bundle_bytes.as_slice());
        assert!(res.is_ok(), "{:?}", res);
        let final_message = res.unwrap();

        let res = responder.process(eik_bytes.as_slice());
        assert!(res.is_ok(), "{:?}", res);
        let res = responder.process(final_message.as_slice());
        assert!(res.is_ok(), res);

        let init = initiator.completed_key_exchange.as_ref().unwrap();
        let resp = responder.completed_key_exchange.as_ref().unwrap();

        let mut vault_ii = vault_i.lock().unwrap();
        let ciphertext_and_tag = vault_ii
            .aead_aes_gcm_encrypt(&init.encrypt_key, b"Hello Alice", &[1u8; 12], &[])
            .unwrap();
        let mut vault_rr = vault_r.lock().unwrap();
        let plaintext = vault_rr
            .aead_aes_gcm_decrypt(
                &resp.decrypt_key,
                ciphertext_and_tag.as_slice(),
                &[1u8; 12],
                &[],
            )
            .unwrap();
        assert_eq!(plaintext, b"Hello Alice");
    }
}
