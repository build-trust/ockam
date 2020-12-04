use super::{CompletedKeyExchange, KeyExchange, KeyExchanger, SHA256_SIZE};
use crate::error::KexExchangeFailError;
use crate::{CipherSuite, NewKeyExchanger, AES_GCM_TAGSIZE};
use ockam_vault::{
    error::{VaultFailError, VaultFailErrorKind},
    types::{
        PublicKey, SecretKey, SecretKeyAttributes, SecretKeyType, SecretPersistenceType,
        SecretPurposeType,
    },
    DynVault, Secret,
};
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;

#[derive(Debug)]
struct KeyPair {
    public_key: PublicKey,
    secret_handle: Box<dyn Secret>,
}

/// Represents the XX Handshake]
struct SymmetricState {
    cipher_suite: CipherSuite,
    identity_key: Option<Arc<Box<dyn Secret>>>,
    identity_public_key: Option<PublicKey>,
    ephemeral_key_pair: Option<KeyPair>,
    remote_static_public_key: Option<PublicKey>,
    remote_ephemeral_public_key: Option<PublicKey>,
    key: Option<Box<dyn Secret>>,
    nonce: u16,
    h: Option<[u8; SHA256_SIZE]>,
    ck: Option<Box<dyn Secret>>,
    vault: Arc<Mutex<dyn DynVault>>,
}

impl Zeroize for SymmetricState {
    fn zeroize(&mut self) {
        self.nonce.zeroize();
        self.h.zeroize();
    }
}

impl std::fmt::Debug for SymmetricState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "SymmetricState {{ cipher_suite: {:?}, key: {:?}, nonce: {:?}, h: {:?}, ck: {:?} }}",
            self.cipher_suite, self.key, self.nonce, self.h, self.ck
        )
    }
}

impl SymmetricState {
    fn get_secret_key_type(&self) -> SecretKeyType {
        match self.cipher_suite {
            CipherSuite::Curve25519AesGcmSha256 => SecretKeyType::Curve25519,
            CipherSuite::P256Aes128GcmSha256 => SecretKeyType::P256,
        }
    }

    fn get_symmetric_key_type(&self) -> SecretKeyType {
        match self.cipher_suite {
            CipherSuite::Curve25519AesGcmSha256 => SecretKeyType::Aes256,
            CipherSuite::P256Aes128GcmSha256 => SecretKeyType::Aes128,
        }
    }

    fn create_public_key(&self, public_key: &[u8]) -> Result<PublicKey, VaultFailError> {
        match self.cipher_suite {
            CipherSuite::Curve25519AesGcmSha256 => {
                if public_key.len() != 32 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSize));
                }
                Ok(PublicKey::Curve25519(*array_ref![public_key, 0, 32]))
            }
            CipherSuite::P256Aes128GcmSha256 => {
                if public_key.len() != 65 {
                    return Err(VaultFailError::from(VaultFailErrorKind::InvalidSize));
                }
                Ok(PublicKey::P256(*array_ref![public_key, 0, 65]))
            }
        }
    }

    fn get_public_key_size(&self) -> usize {
        match self.cipher_suite {
            CipherSuite::Curve25519AesGcmSha256 => 32,
            CipherSuite::P256Aes128GcmSha256 => 65,
        }
    }

    pub fn new(
        cipher_suite: CipherSuite,
        vault: Arc<Mutex<dyn DynVault>>,
        identity_key: Option<Arc<Box<dyn Secret>>>,
    ) -> Self {
        Self {
            cipher_suite,
            identity_key,
            identity_public_key: None,
            ephemeral_key_pair: None,
            remote_static_public_key: None,
            remote_ephemeral_public_key: None,
            key: None,
            nonce: 0,
            h: None,
            ck: None,
            vault,
        }
    }
}

impl KeyExchange for SymmetricState {
    fn get_protocol_name(&self) -> &'static [u8] {
        match self.cipher_suite {
            CipherSuite::Curve25519AesGcmSha256 => b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0",
            CipherSuite::P256Aes128GcmSha256 => b"Noise_XX_P256_AES128GCM_SHA256\0\0",
        }
    }

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let secret_key_type = self.get_secret_key_type();

        let mut attributes = SecretKeyAttributes {
            xtype: secret_key_type,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Persistent,
        };
        // 1. Generate a static key pair for this handshake and set it to `s`
        let mut vault = self.vault.lock().unwrap();
        let identity_key = self.identity_key.take();
        let identity_key = match identity_key {
            None => {
                let static_secret_handle = vault.secret_generate(attributes)?;
                self.identity_public_key =
                    Some(vault.secret_public_key_get(&static_secret_handle)?);
                Arc::new(static_secret_handle)
            }
            Some(ik) => {
                self.identity_public_key = Some(vault.secret_public_key_get(&ik)?);
                ik
            }
        };
        self.identity_key = Some(identity_key);

        attributes.persistence = SecretPersistenceType::Ephemeral;
        // 2. Generate an ephemeral key pair for this handshake and set it to e
        let ephemeral_secret_handle = vault.secret_generate(attributes)?;
        let ephemeral_public_key = vault.secret_public_key_get(&ephemeral_secret_handle)?;
        self.ephemeral_key_pair = Some(KeyPair {
            public_key: ephemeral_public_key,
            secret_handle: ephemeral_secret_handle,
        });

        // 3. Set k to empty, Set n to 0
        // let nonce = 0;
        self.key = None;
        self.nonce = 0;

        // 4. Set h and ck to protocol name
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let mut h = [0u8; SHA256_SIZE];
        h[..self.get_protocol_name().len()].copy_from_slice(self.get_protocol_name());
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(SHA256_SIZE),
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        self.ck = Some(vault.secret_import(&SecretKey::Buffer(h.to_vec()), attributes)?);
        self.h = Some(vault.sha256(&h)?);

        Ok(())
    }

    /// Perform the diffie-hellman computation
    fn dh(
        &mut self,
        secret_handle: &Box<dyn Secret>,
        public_key: PublicKey,
    ) -> Result<(), VaultFailError> {
        let ck = self
            .ck
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut vault = self.vault.lock().unwrap();

        let attributes_ck = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(SHA256_SIZE),
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };

        let symmetric_key_type = self.get_symmetric_key_type();

        let attributes_k = SecretKeyAttributes {
            xtype: symmetric_key_type,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };

        let mut hkdf_output = vault.ec_diffie_hellman_hkdf_sha256(
            secret_handle,
            public_key,
            &ck,
            b"",
            vec![attributes_ck, attributes_k],
        )?;

        if hkdf_output.len() != 2 {
            return Err(VaultFailError::from(VaultFailErrorKind::Ecdh));
        }

        let key = self.key.take();
        if key.is_some() {
            vault.secret_destroy(key.unwrap())?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        vault.secret_destroy(ck)?;
        self.ck = Some(hkdf_output.pop().unwrap());

        self.nonce = 0;

        Ok(())
    }

    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError> {
        let h = &self
            .h
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut input = h.to_vec();
        input.extend_from_slice(data.as_ref());
        let vault = self.vault.lock().unwrap();
        self.h = Some(vault.sha256(&input)?);
        Ok(())
    }

    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        plaintext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let h = &self
            .h
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext_and_tag = {
            let mut vault = self.vault.lock().unwrap();
            vault.aead_aes_gcm_encrypt(
                self.key
                    .as_ref()
                    .ok_or(VaultFailErrorKind::AeadAesGcmEncrypt)?,
                plaintext.as_ref(),
                nonce.as_ref(),
                h,
            )?
        };
        self.mix_hash(&ciphertext_and_tag)?;
        self.nonce += 1;
        Ok(ciphertext_and_tag)
    }

    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let h = &self
            .h
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = {
            let mut vault = self.vault.lock().unwrap();
            vault.aead_aes_gcm_decrypt(
                self.key
                    .as_ref()
                    .ok_or(VaultFailErrorKind::AeadAesGcmDecrypt)?,
                ciphertext,
                nonce.as_ref(),
                h,
            )?
        };
        self.mix_hash(ciphertext)?;
        self.nonce += 1;
        Ok(plaintext)
    }

    /// Split step in Noise protocol
    fn split(&mut self) -> Result<(Box<dyn Secret>, Box<dyn Secret>), VaultFailError> {
        let ck = self
            .ck
            .as_ref()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut vault = self.vault.lock().unwrap();
        let symmetric_key_type = self.get_symmetric_key_type();
        let attributes = SecretKeyAttributes {
            xtype: symmetric_key_type,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        let mut hkdf_output = vault.hkdf_sha256(ck, b"", None, vec![attributes, attributes])?;

        if hkdf_output.len() != 2 {
            return Err(VaultFailError::from(VaultFailErrorKind::HkdfSha256));
        }

        let res1 = hkdf_output.pop().unwrap();
        let res0 = hkdf_output.pop().unwrap();

        Ok((res0, res1))
    }

    /// Set this state up to send and receive messages
    fn finalize(
        self,
        encrypt_key: Box<dyn Secret>,
        decrypt_key: Box<dyn Secret>,
    ) -> Result<CompletedKeyExchange, VaultFailError> {
        let h = self
            .h
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let local_static_secret = self
            .identity_key
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let remote_static_public_key = self
            .remote_static_public_key
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        Ok(CompletedKeyExchange {
            h,
            encrypt_key,
            decrypt_key,
            local_static_secret,
            remote_static_public_key,
        })
    }
}

/// Provides methods for handling the initiator role
#[derive(Debug)]
struct Initiator(SymmetricState);

impl Initiator {
    /// Encode the first message to be sent
    pub fn encode_message_1<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let ephemeral_public_key = self
            .0
            .ephemeral_key_pair
            .as_ref()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?
            .public_key;

        let payload = payload.as_ref();
        self.0.mix_hash(ephemeral_public_key)?;
        self.0.mix_hash(payload)?;

        let mut output = ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub fn decode_message_2<B: AsRef<[u8]>>(
        &mut self,
        message: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let t = &mut self.0;
        let public_key_size = t.get_public_key_size();
        let message = message.as_ref();
        if message.len() < 2 * public_key_size + AES_GCM_TAGSIZE {
            return Err(VaultFailErrorKind::SecretSizeMismatch.into());
        }

        let ephemeral_key_pair = t
            .ephemeral_key_pair
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let ephemeral_secret_handle = &ephemeral_key_pair.secret_handle;

        let mut index_l = 0;
        let mut index_r = public_key_size;
        let re = &message[..index_r];
        index_l += public_key_size;
        index_r += public_key_size + AES_GCM_TAGSIZE;
        let encrypted_rs_and_tag = &message[index_l..index_r];
        let encrypted_payload_and_tag = &message[index_r..];

        let re = t.create_public_key(re)?;

        t.remote_ephemeral_public_key = Some(re);

        t.mix_hash(&re)?;
        t.dh(ephemeral_secret_handle, re)?;
        let rs = t.decrypt_and_mix_hash(encrypted_rs_and_tag)?;
        let rs = t.create_public_key(&rs)?;
        t.remote_static_public_key = Some(rs);
        t.dh(ephemeral_secret_handle, rs)?;

        t.ephemeral_key_pair = Some(ephemeral_key_pair);
        let payload = t.decrypt_and_mix_hash(encrypted_payload_and_tag)?;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub fn encode_message_3<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let t = &mut self.0;
        let static_secret = t
            .identity_key
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let static_public = t
            .identity_public_key
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let remote_ephemeral_public_key = t
            .remote_ephemeral_public_key
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let mut encrypted_s_and_tag = t.encrypt_and_mix_hash(static_public)?;
        t.dh(&static_secret, remote_ephemeral_public_key)?;
        t.identity_key = Some(static_secret);
        let mut encrypted_payload_and_tag = t.encrypt_and_mix_hash(payload)?;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    /// Setup this initiator to send and receive messages
    /// after encoding message 3
    pub fn finalize(mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        let keys = self.0.split()?;
        self.0.finalize(keys.1, keys.0)
    }
}

/// Provides methods for handling the responder role
#[derive(Debug)]
struct Responder(SymmetricState);

impl Responder {
    /// Decode the first message sent
    pub fn decode_message_1<B: AsRef<[u8]>>(
        &mut self,
        message_1: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let public_key_size = self.0.get_public_key_size();
        let message_1 = message_1.as_ref();
        if message_1.len() < public_key_size {
            return Err(VaultFailErrorKind::SecretSizeMismatch.into());
        }

        let re = &message_1[..public_key_size];
        self.0.remote_ephemeral_public_key = Some(self.0.create_public_key(re)?);
        self.0.mix_hash(&re)?;
        self.0.mix_hash(&message_1[public_key_size..])?;
        Ok(message_1[public_key_size..].to_vec())
    }

    /// Encode the second message to be sent
    pub fn encode_message_2<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let t = &mut self.0;
        let static_secret = t
            .identity_key
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;
        let static_public = t
            .identity_public_key
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;
        let ephemeral_key_pair = t
            .ephemeral_key_pair
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;
        let remote_ephemeral_public_key = t
            .remote_ephemeral_public_key
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        t.mix_hash(ephemeral_key_pair.public_key)?;
        t.dh(
            &ephemeral_key_pair.secret_handle,
            remote_ephemeral_public_key,
        )?;

        let mut encrypted_s_and_tag = t.encrypt_and_mix_hash(static_public)?;
        t.dh(&static_secret, remote_ephemeral_public_key)?;
        t.remote_ephemeral_public_key = Some(remote_ephemeral_public_key);
        t.identity_key = Some(static_secret);
        let mut encrypted_payload_and_tag = t.encrypt_and_mix_hash(payload)?;

        let mut output = ephemeral_key_pair.public_key.as_ref().to_vec();
        t.ephemeral_key_pair = Some(ephemeral_key_pair);
        output.append(&mut encrypted_s_and_tag);
        output.append(&mut encrypted_payload_and_tag);
        Ok(output)
    }

    /// Decode the final message received for the handshake
    pub fn decode_message_3<B: AsRef<[u8]>>(
        &mut self,
        message_3: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let t = &mut self.0;
        let public_key_size = t.get_public_key_size();
        let message_3 = message_3.as_ref();
        if message_3.len() < public_key_size + AES_GCM_TAGSIZE {
            return Err(VaultFailErrorKind::SecretSizeMismatch.into());
        }

        let ephemeral_key_pair = t
            .ephemeral_key_pair
            .take()
            .ok_or_else(|| VaultFailError::from(VaultFailErrorKind::InvalidContext))?;

        let rs = t.decrypt_and_mix_hash(&message_3[..public_key_size + AES_GCM_TAGSIZE])?;
        let rs = t.create_public_key(&rs)?;
        t.dh(&ephemeral_key_pair.secret_handle, rs)?;
        t.ephemeral_key_pair = Some(ephemeral_key_pair);
        let payload = t.decrypt_and_mix_hash(&message_3[public_key_size + AES_GCM_TAGSIZE..])?;
        t.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    /// Setup this responder to send and receive messages
    /// after decoding message 3
    pub fn finalize(mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        let keys = self.0.split()?;
        self.0.finalize(keys.0, keys.1)
    }
}

/// The states the connection XX pattern initiator completes
#[derive(Debug)]
enum InitiatorState {
    /// Run encode message 1
    EncodeMessage1,
    /// Run decode message 2
    DecodeMessage2,
    /// Run encode message 3
    EncodeMessage3,
    /// Finished
    Done,
}

/// The states the connection XX pattern responder completes
#[derive(Debug)]
enum ResponderState {
    /// Run decode message 1
    DecodeMessage1,
    /// Run encode message 2
    EncodeMessage2,
    /// Run decode message 3
    DecodeMessage3,
    /// Finished
    Done,
}

/// Represents an XX initiator
#[derive(Debug)]
pub struct XXInitiator {
    state: InitiatorState,
    initiator: Initiator,
    run_prologue: bool,
}

/// Represents an XX NewKeyExchanger
pub struct XXNewKeyExchanger {
    cipher_suite: CipherSuite,
    vault_initiator: Arc<Mutex<dyn DynVault + Send>>,
    vault_responder: Arc<Mutex<dyn DynVault + Send>>,
}

impl std::fmt::Debug for XXNewKeyExchanger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cipher_suite.fmt(f)
    }
}

impl XXNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(
        cipher_suite: CipherSuite,
        vault_initiator: Arc<Mutex<dyn DynVault + Send>>,
        vault_responder: Arc<Mutex<dyn DynVault + Send>>,
    ) -> Self {
        Self {
            cipher_suite,
            vault_initiator,
            vault_responder,
        }
    }
}

impl NewKeyExchanger<XXInitiator, XXResponder> for XXNewKeyExchanger {
    /// Create a new initiator using the provided backing vault
    fn initiator(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> XXInitiator {
        let ss = SymmetricState::new(
            self.cipher_suite,
            self.vault_initiator.clone(),
            identity_key,
        );
        XXInitiator {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(ss),
            run_prologue: true,
        }
    }

    /// Create a new responder using the provided backing vault
    fn responder(&self, identity_key: Option<Arc<Box<dyn Secret>>>) -> XXResponder {
        let ss = SymmetricState::new(
            self.cipher_suite,
            self.vault_responder.clone(),
            identity_key,
        );
        XXResponder {
            state: ResponderState::DecodeMessage1,
            responder: Responder(ss),
            run_prologue: true,
        }
    }
}

/// Represents an XX responder
#[derive(Debug)]
pub struct XXResponder {
    state: ResponderState,
    responder: Responder,
    run_prologue: bool,
}

impl KeyExchanger for XXInitiator {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            InitiatorState::EncodeMessage1 => {
                if self.run_prologue {
                    self.initiator.0.prologue()?;
                }
                let msg = self.initiator.encode_message_1(data)?;
                self.state = InitiatorState::DecodeMessage2;
                Ok(msg)
            }
            InitiatorState::DecodeMessage2 => {
                let msg = self.initiator.decode_message_2(data)?;
                self.state = InitiatorState::EncodeMessage3;
                Ok(msg)
            }
            InitiatorState::EncodeMessage3 => {
                let msg = self.initiator.encode_message_3(data)?;
                self.state = InitiatorState::Done;
                Ok(msg)
            }
            InitiatorState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, InitiatorState::Done)
    }

    fn finalize(self: Box<Self>) -> Result<CompletedKeyExchange, VaultFailError> {
        match self.state {
            InitiatorState::Done => self.initiator.finalize(),
            _ => Err(VaultFailErrorKind::IOError.into()),
        }
    }
}

impl KeyExchanger for XXResponder {
    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            ResponderState::DecodeMessage1 => {
                if self.run_prologue {
                    self.responder.0.prologue()?;
                }
                let msg = self.responder.decode_message_1(data)?;
                self.state = ResponderState::EncodeMessage2;
                Ok(msg)
            }
            ResponderState::EncodeMessage2 => {
                let msg = self.responder.encode_message_2(data)?;
                self.state = ResponderState::DecodeMessage3;
                Ok(msg)
            }
            ResponderState::DecodeMessage3 => {
                let msg = self.responder.decode_message_3(data)?;
                self.state = ResponderState::Done;
                Ok(msg)
            }
            ResponderState::Done => Ok(vec![]),
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self.state, ResponderState::Done)
    }

    fn finalize(self: Box<Self>) -> Result<CompletedKeyExchange, VaultFailError> {
        match self.state {
            ResponderState::Done => self.responder.finalize(),
            _ => Err(VaultFailErrorKind::IOError.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_vault::software::DefaultVault;

    #[test]
    fn prologue() {
        let exp_h = [
            93, 247, 43, 103, 185, 101, 173, 209, 22, 143, 10, 108, 117, 109, 242, 28, 32, 79, 126,
            100, 252, 104, 43, 230, 163, 171, 75, 104, 44, 141, 182, 75,
        ];
        let vault = Arc::new(Mutex::new(DefaultVault::default()));
        let mut state =
            SymmetricState::new(CipherSuite::Curve25519AesGcmSha256, vault.clone(), None);
        let res = state.prologue();
        assert!(res.is_ok());
        assert_eq!(state.h.unwrap(), exp_h);

        let mut vault = vault.lock().unwrap();
        let ck = vault.secret_export(&state.ck.unwrap()).unwrap();

        match &ck {
            SecretKey::Buffer(vec) => {
                assert_eq!(vec.as_slice(), *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0")
            }
            _ => panic!(),
        }

        assert_eq!(state.nonce, 0);
    }

    #[test]
    fn handshake_1() {
        const INIT_STATIC: &str =
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        const INIT_EPH: &str = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        const RESP_STATIC: &str =
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        const RESP_EPH: &str = "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60";
        const MSG_1_CIPHERTEXT: &str =
            "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254";
        const MSG_1_PAYLOAD: &str = "";
        const MSG_2_CIPHERTEXT: &str = "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4ab872c0505b9e824df091b74ffe10a2b32809cab21f";
        const MSG_2_PAYLOAD: &str = "";
        const MSG_3_CIPHERTEXT: &str = "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56";
        const MSG_3_PAYLOAD: &str = "";

        mock_handshake(
            INIT_STATIC,
            INIT_EPH,
            RESP_STATIC,
            RESP_EPH,
            MSG_1_PAYLOAD,
            MSG_1_CIPHERTEXT,
            MSG_2_PAYLOAD,
            MSG_2_CIPHERTEXT,
            MSG_3_PAYLOAD,
            MSG_3_CIPHERTEXT,
        );
    }

    #[test]
    fn handshake_2() {
        const INIT_STATIC: &str =
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        const RESP_STATIC: &str =
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        const INIT_EPH: &str = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        const RESP_EPH: &str = "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60";
        const MSG_1_PAYLOAD: &str = "746573745f6d73675f30";
        const MSG_1_CIPHERTEXT: &str =
            "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254746573745f6d73675f30";
        const MSG_2_PAYLOAD: &str = "746573745f6d73675f31";
        const MSG_2_CIPHERTEXT: &str = "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8c9f29dcec8d3ab554f4a5330657867fe4917917195c8cf360e08d6dc5f71baf875ec6e3bfc7afda4c9c2";
        const MSG_3_PAYLOAD: &str = "746573745f6d73675f32";
        const MSG_3_CIPHERTEXT: &str = "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40232c55cd96d1350af861f6a04978f7d5e070c07602c6b84d25a331242a71c50ae31dd4c164267fd48bd2";

        mock_handshake(
            INIT_STATIC,
            INIT_EPH,
            RESP_STATIC,
            RESP_EPH,
            MSG_1_PAYLOAD,
            MSG_1_CIPHERTEXT,
            MSG_2_PAYLOAD,
            MSG_2_CIPHERTEXT,
            MSG_3_PAYLOAD,
            MSG_3_CIPHERTEXT,
        );
    }

    #[test]
    fn handshake_main() {
        const INIT_STATIC: &str =
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        const INIT_EPH: &str = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        const RESP_STATIC: &str =
            "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        const RESP_EPH: &str = "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60";
        const MSG_1_CIPHERTEXT: &str =
            "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254";
        const MSG_1_PAYLOAD: &str = "";
        const MSG_2_CIPHERTEXT: &str = "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4ab872c0505b9e824df091b74ffe10a2b32809cab21f";
        const MSG_2_PAYLOAD: &str = "";
        const MSG_3_CIPHERTEXT: &str = "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56";
        const MSG_3_PAYLOAD: &str = "";

        let vault_init = Arc::new(Mutex::new(DefaultVault::default()));
        let vault_resp = Arc::new(Mutex::new(DefaultVault::default()));

        let ss_init = mock_prologue(vault_init.clone(), INIT_STATIC, INIT_EPH);
        let ss_resp = mock_prologue(vault_resp.clone(), RESP_STATIC, RESP_EPH);
        let mut initiator = XXInitiator {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(ss_init),
            run_prologue: false,
        };
        let mut responder = XXResponder {
            state: ResponderState::DecodeMessage1,
            responder: Responder(ss_resp),
            run_prologue: false,
        };

        assert!(!initiator.is_complete());
        assert!(!responder.is_complete());
        let res = responder.process(&[]);
        assert!(res.is_err());
        let res = initiator.process(&hex::decode(MSG_1_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(hex::encode(&msg1), MSG_1_CIPHERTEXT);

        let res = responder.process(&msg1);
        assert!(res.is_ok());
        let res = responder.process(&hex::decode(MSG_2_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(hex::encode(&msg2), MSG_2_CIPHERTEXT);

        let res = initiator.process(&msg2);
        assert!(res.is_ok());
        let res = initiator.process(&hex::decode(MSG_3_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(hex::encode(&msg3), MSG_3_CIPHERTEXT);

        let res = responder.process(&msg3);
        assert!(res.is_ok());

        let initiator = Box::new(initiator);
        let res = initiator.finalize();
        assert!(res.is_ok());
        let alice = res.unwrap();
        let responder = Box::new(responder);
        let res = responder.finalize();
        assert!(res.is_ok());
        let bob = res.unwrap();
        assert_eq!(alice.h, bob.h);
        let mut vault_in = vault_init.lock().unwrap();
        let res =
            vault_in.aead_aes_gcm_encrypt(&alice.encrypt_key, b"hello bob", &[0u8; 12], &alice.h);

        assert!(res.is_ok());
        let ciphertext = res.unwrap();
        let mut vault_re = vault_resp.lock().unwrap();

        let res = vault_re.aead_aes_gcm_decrypt(&bob.decrypt_key, &ciphertext, &[0u8; 12], &bob.h);
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, b"hello bob");

        let res =
            vault_re.aead_aes_gcm_encrypt(&bob.encrypt_key, b"hello alice", &[1u8; 12], &bob.h);
        assert!(res.is_ok());
        let ciphertext = res.unwrap();
        let res =
            vault_in.aead_aes_gcm_decrypt(&alice.decrypt_key, &ciphertext, &[1u8; 12], &alice.h);
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, b"hello alice");
    }

    fn mock_handshake(
        init_static: &str,
        init_eph: &str,
        resp_static: &str,
        resp_eph: &str,
        msg_1_payload: &str,
        msg_1_ciphertext: &str,
        msg_2_payload: &str,
        msg_2_ciphertext: &str,
        msg_3_payload: &str,
        msg_3_ciphertext: &str,
    ) {
        let vault_init = Arc::new(Mutex::new(DefaultVault::default()));
        let vault_resp = Arc::new(Mutex::new(DefaultVault::default()));

        let ss_init = mock_prologue(vault_init.clone(), init_static, init_eph);
        let ss_resp = mock_prologue(vault_resp.clone(), resp_static, resp_eph);
        let mut initiator = Initiator(ss_init);
        let mut responder = Responder(ss_resp);

        let res = initiator.encode_message_1(hex::decode(msg_1_payload).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(hex::encode(&msg1), msg_1_ciphertext);

        let res = responder.decode_message_1(msg1);
        assert!(res.is_ok());

        let res = responder.encode_message_2(hex::decode(msg_2_payload).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(hex::encode(&msg2), msg_2_ciphertext);

        let res = initiator.decode_message_2(msg2);
        assert!(res.is_ok());
        let res = initiator.encode_message_3(hex::decode(msg_3_payload).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(hex::encode(&msg3), msg_3_ciphertext);

        let res = responder.decode_message_3(msg3);
        assert!(res.is_ok());

        let res = initiator.finalize();
        assert!(res.is_ok());
        let res = responder.finalize();
        assert!(res.is_ok());
    }

    fn mock_prologue(
        vault_mutex: Arc<Mutex<DefaultVault>>,
        static_private: &str,
        ephemeral_private: &str,
    ) -> SymmetricState {
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        // Static x25519 for this handshake, `s`
        let bytes = hex::decode(static_private).unwrap();
        let key = SecretKey::Curve25519(*array_ref![bytes, 0, 32]);
        let mut vault = vault_mutex.lock().unwrap();
        let static_secret_handle = vault.secret_import(&key, attributes).unwrap();
        let static_public_key = vault.secret_public_key_get(&static_secret_handle).unwrap();

        // Ephemeral x25519 for this handshake, `e`
        let bytes = hex::decode(ephemeral_private).unwrap();
        let key = SecretKey::Curve25519(*array_ref![bytes, 0, 32]);
        let ephemeral_secret_handle = vault.secret_import(&key, attributes).unwrap();
        let ephemeral_public_key = vault
            .secret_public_key_get(&ephemeral_secret_handle)
            .unwrap();

        // 3. Set k to empty, Set n to 0
        let nonce = 0;

        // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let h = vault
            .sha256(b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0")
            .unwrap();
        let ck = *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0";

        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Buffer(ck.len()),
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        let ck = vault
            .secret_import(&SecretKey::Buffer(ck.to_vec()), attributes)
            .unwrap();
        SymmetricState {
            cipher_suite: CipherSuite::Curve25519AesGcmSha256,
            identity_public_key: Some(static_public_key),
            ephemeral_key_pair: Some(KeyPair {
                public_key: ephemeral_public_key,
                secret_handle: ephemeral_secret_handle,
            }),
            remote_ephemeral_public_key: None,
            remote_static_public_key: None,
            identity_key: Some(Arc::new(static_secret_handle)),
            key: None,
            nonce,
            h: Some(h),
            ck: Some(ck),
            vault: vault_mutex.clone(),
        }
    }
}
