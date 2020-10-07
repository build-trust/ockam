use crate::error::{KexExchangeFailError, KeyExchangeFailErrorKind};
use crate::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_vault::types::{
    SecretKey, SecretKeyAttributes, SecretKeyType, SecretPersistenceType, SecretPurposeType,
};
use ockam_vault::{
    error::VaultFailError,
    types::{PublicKey, SecretKeyContext},
    DynVault,
};
use std::{
    convert::TryFrom,
    sync::{Arc, Mutex},
};
use subtle::ConstantTimeEq;

/// Represents and (X)EdDSA or ECDSA signature
/// from Ed25519 or P-256
#[derive(Clone, Copy, Debug)]
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

/// Represents all the keys and signature to send to an enrollee
#[derive(Clone, Copy, Debug)]
pub struct PreKeyBundle {
    identity_key: PublicKey,
    signed_prekey: PublicKey,
    signature_prekey: Signature,
    one_time_prekey: PublicKey,
}

impl PreKeyBundle {
    const MIN_SIZE: usize = 32 + 32 + 64 + 32;
    /// Convert the prekey bundle to a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(self.identity_key.as_ref());
        output.extend_from_slice(self.signed_prekey.as_ref());
        output.extend_from_slice(self.signature_prekey.as_ref());
        output.extend_from_slice(self.one_time_prekey.as_ref());
        output
    }
}

impl TryFrom<&[u8]> for PreKeyBundle {
    type Error = KexExchangeFailError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::MIN_SIZE {
            return Err(
                KeyExchangeFailErrorKind::InvalidByteCount(Self::MIN_SIZE, data.len()).into(),
            );
        }
        let identity_key = PublicKey::Curve25519(*array_ref![data, 0, 32]);
        let signed_prekey = PublicKey::Curve25519(*array_ref![data, 32, 32]);
        let signature_prekey = Signature(*array_ref![data, 64, 64]);
        let one_time_prekey = PublicKey::Curve25519(*array_ref![data, 128, 32]);
        Ok(Self {
            identity_key,
            signed_prekey,
            signature_prekey,
            one_time_prekey,
        })
    }
}

#[derive(Debug)]
enum InitiatorState {
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

/// The initiator of X3DH creates a prekey bundle that can be used to establish a shared
/// secret key with another party that can use
pub struct X3dhInitiator {
    identity_key: SecretKeyContext,
    signed_prekey: SecretKeyContext,
    one_time_prekey: SecretKeyContext,
    expected_enrollment_key: Option<PublicKey>,
    state: InitiatorState,
    vault: Arc<Mutex<dyn DynVault + Send>>,
    completed_key_exchange: Option<CompletedKeyExchange>,
}

impl X3dhInitiator {
    fn new(v: Arc<Mutex<dyn DynVault + Send>>) -> Self {
        Self {
            identity_key: SecretKeyContext::Memory(0),
            signed_prekey: SecretKeyContext::Memory(0),
            one_time_prekey: SecretKeyContext::Memory(0),
            expected_enrollment_key: None,
            completed_key_exchange: None,
            state: InitiatorState::GenerateBundle,
            vault: v,
        }
    }

    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let mut vault = self.vault.lock().unwrap();
        let p_atts = SecretKeyAttributes {
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Persistent,
            xtype: SecretKeyType::Curve25519,
        };
        let e_atts = SecretKeyAttributes {
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
            xtype: SecretKeyType::Curve25519,
        };
        self.identity_key = vault.secret_generate(p_atts)?;
        self.signed_prekey = vault.secret_generate(p_atts)?;
        self.one_time_prekey = vault.secret_generate(e_atts)?;
        self.expected_enrollment_key = None;
        self.completed_key_exchange = None;
        Ok(())
    }
}

impl std::fmt::Debug for X3dhInitiator {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            r#"X3dhInitiator {{ identity_key: {:?},
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
enum ResponderState {
    GenerateEphemeralIdentityKey,
    ProcessPreKeyBundle,
    Done,
}

/// The responder of X3DH receives a prekey bundle and computes the shared secret
/// to communicate the first message to the initiator
#[derive(Clone)]
pub struct X3dhResponder {
    ephemeral_identity_key: SecretKeyContext,
    prekey_bundle: Option<PreKeyBundle>,
    state: ResponderState,
    vault: Arc<Mutex<dyn DynVault + Send>>,
    completed_key_exchange: Option<CompletedKeyExchange>,
}

impl X3dhResponder {
    fn new(v: Arc<Mutex<dyn DynVault + Send>>) -> Self {
        Self {
            ephemeral_identity_key: SecretKeyContext::Memory(0),
            prekey_bundle: None,
            state: ResponderState::GenerateEphemeralIdentityKey,
            vault: v,
            completed_key_exchange: None,
        }
    }

    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let mut vault = self.vault.lock().unwrap();
        let p_atts = SecretKeyAttributes {
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Persistent,
            xtype: SecretKeyType::Curve25519,
        };
        self.ephemeral_identity_key = vault.secret_generate(p_atts)?;
        self.prekey_bundle = None;
        self.completed_key_exchange = None;
        Ok(())
    }
}

impl std::fmt::Debug for X3dhResponder {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            r#"X3dhResponder {{ ephemeral_identity_key: {:?}, prekey_bundle: {:?}, state: {:?}, vault, completed_key_exchange: {:?} }}"#,
            self.ephemeral_identity_key,
            self.prekey_bundle,
            self.state,
            self.completed_key_exchange
        )
    }
}

impl KeyExchanger for X3dhInitiator {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            InitiatorState::GenerateBundle => {
                self.prologue()?;
                let mut vault = self.vault.lock().unwrap();
                let signed_prekey = vault.secret_public_key_get(self.signed_prekey)?;
                let signature = vault.sign(self.identity_key, signed_prekey.as_ref())?;
                let identity_key = vault.secret_public_key_get(self.identity_key)?;
                let one_time_prekey = vault.secret_public_key_get(self.one_time_prekey)?;
                let bundle = PreKeyBundle {
                    identity_key,
                    signed_prekey,
                    signature_prekey: Signature(signature),
                    one_time_prekey,
                };
                self.state = InitiatorState::SetEnrollmentKey;
                Ok(bundle.to_bytes())
            }
            InitiatorState::SetEnrollmentKey => {
                if data.len() != 32 {
                    return Err(KeyExchangeFailErrorKind::InvalidByteCount(32, data.len()).into());
                }
                self.expected_enrollment_key =
                    Some(PublicKey::Curve25519(*array_ref![data, 0, 32]));
                self.state = InitiatorState::VerifyEnrollment;
                Ok(vec![])
            }
            InitiatorState::VerifyEnrollment => {
                debug_assert!(self.expected_enrollment_key.is_some());
                if data.len() != ENROLLMENT_MSG_SIZE {
                    return Err(KeyExchangeFailErrorKind::InvalidByteCount(
                        ENROLLMENT_MSG_SIZE,
                        data.len(),
                    )
                    .into());
                }
                let mut vault = self.vault.lock().unwrap();
                let eik = self.expected_enrollment_key.as_ref().unwrap();
                let id = vault.sha256(eik.as_ref())?;
                if id.ct_eq(&data[32..64]).unwrap_u8() != 1 {
                    return Err(KeyExchangeFailErrorKind::InvalidHash {
                        expected: hex::encode(id),
                        actual: hex::encode(&data[32..64]),
                    }
                    .into());
                }
                let ek = PublicKey::Curve25519(*array_ref![data, 0, 32]);

                let dh1 = vault.ec_diffie_hellman(self.signed_prekey, *eik)?;
                let dh2 = vault.ec_diffie_hellman(self.identity_key, ek)?;
                let dh3 = vault.ec_diffie_hellman(self.signed_prekey, ek)?;
                let dh4 = vault.ec_diffie_hellman(self.one_time_prekey, ek)?;
                let mut ikm = vec![0u8; 32];
                ikm.extend_from_slice(vault.secret_export(dh1)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh2)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh3)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh4)?.as_ref());
                let mut sk = vault.hkdf_sha256(&[0u8; 32], CSUITE, ikm.as_slice(), 64)?;
                let tek = *array_ref![sk, 0, 32];
                let tdk = *array_ref![sk, 32, 32];
                let attributes = SecretKeyAttributes {
                    xtype: SecretKeyType::Aes256,
                    purpose: SecretPurposeType::KeyAgreement,
                    persistence: SecretPersistenceType::Persistent,
                };
                let encrypt_key = vault.secret_import(&SecretKey::Aes256(tek), attributes)?;
                let decrypt_key = vault.secret_import(&SecretKey::Aes256(tdk), attributes)?;
                let mut state_hash = vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut sk);
                let state_hash = vault.sha256(state_hash.as_slice())?;

                let mut aad = data[..64].to_vec();
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);
                //TODO: get the channel address from the message somehow if needed
                let plaintext = vault.aead_aes_gcm_decrypt(
                    decrypt_key,
                    &data[64..],
                    &data[..12],
                    aad.as_slice(),
                )?;
                let ikb = PublicKey::Curve25519(*array_ref![plaintext, 0, 32]);
                let signature = *array_ref![plaintext, 32, 64];
                vault.verify(signature, *eik, &plaintext[..32])?;

                self.completed_key_exchange = Some(CompletedKeyExchange {
                    h: state_hash,
                    encrypt_key,
                    decrypt_key,
                    local_static_secret: self.identity_key,
                    remote_static_public_key: ikb,
                });
                self.state = InitiatorState::Done;
                Ok(vec![])
            }
            InitiatorState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, InitiatorState::Done)
    }

    fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        Ok(*self.completed_key_exchange.as_ref().unwrap())
    }
}

impl NewKeyExchanger<Self, X3dhResponder> for X3dhInitiator {
    fn initiator(v: Arc<Mutex<dyn DynVault + Send>>) -> X3dhInitiator {
        Self::new(v)
    }

    fn responder(v: Arc<Mutex<dyn DynVault + Send>>) -> X3dhResponder {
        X3dhResponder::new(v)
    }
}

impl KeyExchanger for X3dhResponder {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            ResponderState::GenerateEphemeralIdentityKey => {
                self.prologue()?;
                let mut vault = self.vault.lock().unwrap();
                self.ephemeral_identity_key = vault.secret_generate(SecretKeyAttributes {
                    purpose: SecretPurposeType::KeyAgreement,
                    persistence: SecretPersistenceType::Persistent,
                    xtype: SecretKeyType::Curve25519,
                })?;
                let pubkey = vault.secret_public_key_get(self.ephemeral_identity_key)?;
                self.state = ResponderState::ProcessPreKeyBundle;
                Ok(pubkey.as_ref().to_vec())
            }
            ResponderState::ProcessPreKeyBundle => {
                let prekey_bundle = PreKeyBundle::try_from(data)?;

                let mut vault = self.vault.lock().unwrap();

                // Check the prekey_bundle signature
                vault.verify(
                    prekey_bundle.signature_prekey.0,
                    prekey_bundle.identity_key,
                    prekey_bundle.signed_prekey.as_ref(),
                )?;

                let mut atts = SecretKeyAttributes {
                    purpose: SecretPurposeType::KeyAgreement,
                    persistence: SecretPersistenceType::Ephemeral,
                    xtype: SecretKeyType::Curve25519,
                };
                let esk = vault.secret_generate(atts)?;
                let dh1 = vault
                    .ec_diffie_hellman(self.ephemeral_identity_key, prekey_bundle.signed_prekey)?;
                let dh2 = vault.ec_diffie_hellman(esk, prekey_bundle.identity_key)?;
                let dh3 = vault.ec_diffie_hellman(esk, prekey_bundle.signed_prekey)?;
                let dh4 = vault.ec_diffie_hellman(esk, prekey_bundle.one_time_prekey)?;
                let mut ikm = vec![0u8; 32];
                ikm.extend_from_slice(vault.secret_export(dh1)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh2)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh3)?.as_ref());
                ikm.extend_from_slice(vault.secret_export(dh4)?.as_ref());
                let mut sk = vault.hkdf_sha256(&[0u8; 32], CSUITE, ikm.as_slice(), 64)?;
                let tdk = *array_ref![sk, 0, 32];
                let tek = *array_ref![sk, 32, 32];
                let attributes = SecretKeyAttributes {
                    xtype: SecretKeyType::Aes256,
                    purpose: SecretPurposeType::KeyAgreement,
                    persistence: SecretPersistenceType::Persistent,
                };
                let encrypt_key = vault.secret_import(&SecretKey::Aes256(tek), attributes)?;
                let decrypt_key = vault.secret_import(&SecretKey::Aes256(tdk), attributes)?;
                let ek = vault.secret_public_key_get(esk)?;
                let pubkey = vault.secret_public_key_get(self.ephemeral_identity_key)?;

                let mut state_hash = vault.sha256(CSUITE)?.to_vec();
                state_hash.append(&mut sk);
                let state_hash = vault.sha256(state_hash.as_slice())?;

                let mut aad = ek.as_ref().to_vec();
                aad.extend_from_slice(&vault.sha256(pubkey.as_ref())?);
                aad.extend_from_slice(CSUITE);
                aad.extend_from_slice(&state_hash);

                atts.persistence = SecretPersistenceType::Persistent;
                let skb = vault.secret_generate(atts)?;
                let ikb = vault.secret_public_key_get(skb)?;

                let mut plaintext = ikb.as_ref().to_vec();
                plaintext
                    .extend_from_slice(&vault.sign(self.ephemeral_identity_key, ikb.as_ref())?);

                let ciphertext_and_tag = vault.aead_aes_gcm_encrypt(
                    encrypt_key,
                    plaintext.as_slice(),
                    &ek.as_ref()[..12],
                    aad.as_slice(),
                )?;
                self.completed_key_exchange = Some(CompletedKeyExchange {
                    h: state_hash,
                    encrypt_key,
                    decrypt_key,
                    local_static_secret: skb,
                    remote_static_public_key: prekey_bundle.identity_key,
                });
                self.state = ResponderState::Done;
                Ok(ciphertext_and_tag)
            }
            ResponderState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        Ok(*self.completed_key_exchange.as_ref().unwrap())
    }
}

impl NewKeyExchanger<X3dhInitiator, Self> for X3dhResponder {
    fn initiator(v: Arc<Mutex<dyn DynVault + Send>>) -> X3dhInitiator {
        X3dhInitiator::new(v)
    }

    fn responder(v: Arc<Mutex<dyn DynVault + Send>>) -> X3dhResponder {
        Self::new(v)
    }
}
