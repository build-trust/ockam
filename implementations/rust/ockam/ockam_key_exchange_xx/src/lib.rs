use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_vault_core::{
    AsymmetricVault, HashVault, PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType,
    SecretVault, SymmetricVault, AES256_SECRET_LENGTH, CURVE25519_SECRET_LENGTH,
};
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;

mod error;
pub use error::*;

/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE: usize = 16;

#[derive(Debug)]
struct KeyPair {
    public_key: PublicKey,
    secret_handle: Secret,
}

/// Vault with XX required functionality
pub trait XXVault: SecretVault + HashVault + AsymmetricVault + SymmetricVault + Send {}

impl<D> XXVault for D where D: SecretVault + HashVault + AsymmetricVault + SymmetricVault + Send {}

/// Represents the XX Handshake
pub struct SymmetricState {
    identity_key: Option<Secret>,
    identity_public_key: Option<PublicKey>,
    ephemeral_key_pair: Option<KeyPair>,
    remote_static_public_key: Option<PublicKey>,
    remote_ephemeral_public_key: Option<PublicKey>,
    key: Option<Secret>,
    nonce: u16,
    h: Option<[u8; SHA256_SIZE]>,
    ck: Option<Secret>,
    vault: Arc<Mutex<dyn XXVault>>,
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
            "SymmetricState {{ key: {:?}, nonce: {:?}, h: {:?}, ck: {:?} }}",
            self.key, self.nonce, self.h, self.ck
        )
    }
}

impl SymmetricState {
    pub fn new(vault: Arc<Mutex<dyn XXVault>>) -> Self {
        Self {
            identity_key: None,
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

impl SymmetricState {
    fn get_secret_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Curve25519, CURVE25519_SECRET_LENGTH)
    }

    fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }

    fn get_public_key_size(&self) -> usize {
        32
    }

    fn get_protocol_name(&self) -> &'static [u8] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> ockam_core::Result<()> {
        let asymmetric_secret_info = self.get_secret_key_type_and_length();

        let attributes = SecretAttributes::new(
            asymmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            asymmetric_secret_info.1,
        );
        // 1. Generate a static key pair for this handshake and set it to `s`
        let mut vault = self.vault.lock().unwrap();
        let identity_key = self.identity_key.take();
        let identity_key = match identity_key {
            None => {
                let static_secret_handle = vault.secret_generate(attributes)?;
                self.identity_public_key =
                    Some(vault.secret_public_key_get(&static_secret_handle)?);
                static_secret_handle
            }
            Some(ik) => {
                self.identity_public_key = Some(vault.secret_public_key_get(&ik)?);
                ik
            }
        };
        self.identity_key = Some(identity_key);

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
        let attributes = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE,
        );
        self.ck = Some(vault.secret_import(&h, attributes)?);
        self.h = Some(vault.sha256(&h)?);

        Ok(())
    }

    /// Perform the diffie-hellman computation
    fn dh(&mut self, secret_handle: &Secret, public_key: &[u8]) -> ockam_core::Result<()> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

        let mut vault = self.vault.lock().unwrap();

        let attributes_ck = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE,
        );

        let symmetric_secret_info = self.get_symmetric_key_type_and_length();

        let attributes_k = SecretAttributes::new(
            symmetric_secret_info.0,
            SecretPersistence::Ephemeral,
            symmetric_secret_info.1,
        );

        let ecdh = vault.ec_diffie_hellman(secret_handle, public_key)?;

        let mut hkdf_output =
            vault.hkdf_sha256(&ck, b"", Some(&ecdh), vec![attributes_ck, attributes_k])?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let key = self.key.take();
        if key.is_some() {
            vault.secret_destroy(key.unwrap())?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        vault.secret_destroy(self.ck.take().unwrap())?;
        self.ck = Some(hkdf_output.pop().unwrap());

        self.nonce = 0;

        Ok(())
    }

    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> ockam_core::Result<()> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

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
    ) -> ockam_core::Result<Vec<u8>> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext_and_tag = {
            let mut vault = self.vault.lock().unwrap();
            let key = self.key.as_ref().ok_or(XXError::InvalidState)?;
            vault.aead_aes_gcm_encrypt(key, plaintext.as_ref(), nonce.as_ref(), h)?
        };
        self.mix_hash(&ciphertext_and_tag)?;
        self.nonce += 1;
        Ok(ciphertext_and_tag)
    }

    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = {
            let mut vault = self.vault.lock().unwrap();
            let key = self.key.as_ref().ok_or(XXError::InvalidState)?;
            vault.aead_aes_gcm_decrypt(key, ciphertext, nonce.as_ref(), h)?
        };
        self.mix_hash(ciphertext)?;
        self.nonce += 1;
        Ok(plaintext)
    }

    /// Split step in Noise protocol
    fn split(&mut self) -> ockam_core::Result<(Secret, Secret)> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

        let mut vault = self.vault.lock().unwrap();
        let symmetric_key_info = self.get_symmetric_key_type_and_length();
        let attributes = SecretAttributes::new(
            symmetric_key_info.0,
            SecretPersistence::Ephemeral,
            symmetric_key_info.1,
        );
        let mut hkdf_output = vault.hkdf_sha256(ck, b"", None, vec![attributes, attributes])?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let res1 = hkdf_output.pop().unwrap();
        let res0 = hkdf_output.pop().unwrap();

        Ok((res0, res1))
    }

    /// Set this state up to send and receive messages
    fn finalize(
        self,
        encrypt_key: Secret,
        decrypt_key: Secret,
    ) -> ockam_core::Result<CompletedKeyExchange> {
        let h = self.h.ok_or(XXError::InvalidState)?;

        let local_static_secret = self.identity_key.ok_or(XXError::InvalidState)?;

        let remote_static_public_key =
            self.remote_static_public_key.ok_or(XXError::InvalidState)?;

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
    pub fn encode_message_1<B: AsRef<[u8]>>(&mut self, payload: B) -> ockam_core::Result<Vec<u8>> {
        let ephemeral_public_key = self
            .0
            .ephemeral_key_pair
            .as_ref()
            .ok_or(XXError::InvalidState)?
            .public_key
            .clone();

        let payload = payload.as_ref();
        self.0.mix_hash(ephemeral_public_key.as_ref())?;
        self.0.mix_hash(payload)?;

        let mut output = ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub fn decode_message_2<B: AsRef<[u8]>>(&mut self, message: B) -> ockam_core::Result<Vec<u8>> {
        let t = &mut self.0;
        let public_key_size = t.get_public_key_size();
        let message = message.as_ref();
        if message.len() < 2 * public_key_size + AES_GCM_TAGSIZE {
            return Err(XXError::MessageLenMismatch.into());
        }

        let ephemeral_key_pair = t.ephemeral_key_pair.take().ok_or(XXError::InvalidState)?;

        let ephemeral_secret_handle = &ephemeral_key_pair.secret_handle;

        let mut index_l = 0;
        let mut index_r = public_key_size;
        let re = &message[..index_r];
        let re = PublicKey::new(re.to_vec());
        index_l += public_key_size;
        index_r += public_key_size + AES_GCM_TAGSIZE;
        let encrypted_rs_and_tag = &message[index_l..index_r];
        let encrypted_payload_and_tag = &message[index_r..];

        t.mix_hash(re.as_ref())?;
        t.dh(ephemeral_secret_handle, re.as_ref())?;
        t.remote_ephemeral_public_key = Some(re);
        let rs = t.decrypt_and_mix_hash(encrypted_rs_and_tag)?;
        let rs = PublicKey::new(rs);
        t.dh(ephemeral_secret_handle, rs.as_ref())?;
        t.remote_static_public_key = Some(rs);

        t.ephemeral_key_pair = Some(ephemeral_key_pair);
        let payload = t.decrypt_and_mix_hash(encrypted_payload_and_tag)?;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub fn encode_message_3<B: AsRef<[u8]>>(&mut self, payload: B) -> ockam_core::Result<Vec<u8>> {
        let t = &mut self.0;
        let static_secret = t.identity_key.take().ok_or(XXError::InvalidState)?;

        let static_public = t.identity_public_key.clone().ok_or(XXError::InvalidState)?;

        let remote_ephemeral_public_key = t
            .remote_ephemeral_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let mut encrypted_s_and_tag = t.encrypt_and_mix_hash(static_public.as_ref())?;
        t.dh(&static_secret, remote_ephemeral_public_key.as_ref())?;
        t.identity_key = Some(static_secret);
        let mut encrypted_payload_and_tag = t.encrypt_and_mix_hash(payload)?;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    /// Setup this initiator to send and receive messages
    /// after encoding message 3
    pub fn finalize(mut self) -> ockam_core::Result<CompletedKeyExchange> {
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
    ) -> ockam_core::Result<Vec<u8>> {
        let public_key_size = self.0.get_public_key_size();
        let message_1 = message_1.as_ref();
        if message_1.len() < public_key_size {
            return Err(XXError::MessageLenMismatch.into());
        }

        let re = &message_1[..public_key_size];
        let re = PublicKey::new(re.to_vec());
        self.0.mix_hash(re.as_ref())?;
        self.0.mix_hash(&message_1[public_key_size..])?;
        self.0.remote_ephemeral_public_key = Some(re);
        Ok(message_1[public_key_size..].to_vec())
    }

    /// Encode the second message to be sent
    pub fn encode_message_2<B: AsRef<[u8]>>(&mut self, payload: B) -> ockam_core::Result<Vec<u8>> {
        let t = &mut self.0;
        let static_secret = t.identity_key.take().ok_or(XXError::InvalidState)?;
        let static_public = t.identity_public_key.clone().ok_or(XXError::InvalidState)?;
        let ephemeral_key_pair = t.ephemeral_key_pair.take().ok_or(XXError::InvalidState)?;
        let remote_ephemeral_public_key = t
            .remote_ephemeral_public_key
            .take()
            .ok_or(XXError::InvalidState)?;

        t.mix_hash(ephemeral_key_pair.public_key.as_ref())?;
        t.dh(
            &ephemeral_key_pair.secret_handle,
            remote_ephemeral_public_key.as_ref(),
        )?;

        let mut encrypted_s_and_tag = t.encrypt_and_mix_hash(static_public.as_ref())?;
        t.dh(&static_secret, remote_ephemeral_public_key.as_ref())?;
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
    ) -> ockam_core::Result<Vec<u8>> {
        let t = &mut self.0;
        let public_key_size = t.get_public_key_size();
        let message_3 = message_3.as_ref();
        if message_3.len() < public_key_size + AES_GCM_TAGSIZE {
            return Err(XXError::MessageLenMismatch.into());
        }

        let ephemeral_key_pair = t.ephemeral_key_pair.take().ok_or(XXError::InvalidState)?;

        let rs = t.decrypt_and_mix_hash(&message_3[..public_key_size + AES_GCM_TAGSIZE])?;
        let rs = PublicKey::new(rs);
        t.dh(&ephemeral_key_pair.secret_handle, rs.as_ref())?;
        t.ephemeral_key_pair = Some(ephemeral_key_pair);
        let payload = t.decrypt_and_mix_hash(&message_3[public_key_size + AES_GCM_TAGSIZE..])?;
        t.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    /// Setup this responder to send and receive messages
    /// after decoding message 3
    pub fn finalize(mut self) -> ockam_core::Result<CompletedKeyExchange> {
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

impl XXInitiator {
    pub fn new(symmetric_state: SymmetricState, run_prologue: bool) -> Self {
        XXInitiator {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(symmetric_state),
            run_prologue,
        }
    }
}

/// Represents an XX NewKeyExchanger
pub struct XXNewKeyExchanger {
    vault_initiator: Arc<Mutex<dyn XXVault>>,
    vault_responder: Arc<Mutex<dyn XXVault>>,
}

impl XXNewKeyExchanger {
    /// Create a new XXNewKeyExchanger
    pub fn new(
        vault_initiator: Arc<Mutex<dyn XXVault>>,
        vault_responder: Arc<Mutex<dyn XXVault>>,
    ) -> Self {
        Self {
            vault_initiator,
            vault_responder,
        }
    }
}

impl NewKeyExchanger<XXInitiator, XXResponder> for XXNewKeyExchanger {
    /// Create a new initiator using the provided backing vault
    fn initiator(&self) -> XXInitiator {
        let ss = SymmetricState::new(self.vault_initiator.clone());
        XXInitiator {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(ss),
            run_prologue: true,
        }
    }

    /// Create a new responder using the provided backing vault
    fn responder(&self) -> XXResponder {
        let ss = SymmetricState::new(self.vault_responder.clone());
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

impl XXResponder {
    pub fn new(symmetric_state: SymmetricState, run_prologue: bool) -> Self {
        XXResponder {
            state: ResponderState::DecodeMessage1,
            responder: Responder(symmetric_state),
            run_prologue,
        }
    }
}

impl KeyExchanger for XXInitiator {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
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

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        match self.state {
            InitiatorState::Done => self.initiator.finalize(),
            _ => Err(XXError::InvalidState.into()),
        }
    }
}

impl KeyExchanger for XXResponder {
    fn process(&mut self, data: &[u8]) -> ockam_core::Result<Vec<u8>> {
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

    fn finalize(self) -> ockam_core::Result<CompletedKeyExchange> {
        match self.state {
            ResponderState::Done => self.responder.finalize(),
            _ => Err(XXError::InvalidState.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_vault::SoftwareVault;

    #[allow(non_snake_case)]
    #[test]
    fn full_flow__correct_credentials__keys_should_match() {
        let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
        let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
        let key_exchanger =
            XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());

        let mut initiator = key_exchanger.initiator();
        let mut responder = key_exchanger.responder();

        let m1 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m1).unwrap();
        let m2 = responder.process(&[]).unwrap();
        let _ = initiator.process(&m2).unwrap();
        let m3 = initiator.process(&[]).unwrap();
        let _ = responder.process(&m3).unwrap();

        let initiator = Box::new(initiator);
        let initiator = initiator.finalize().unwrap();
        let responder = Box::new(responder);
        let responder = responder.finalize().unwrap();

        let mut vault_in = vault_initiator.lock().unwrap();
        let mut vault_re = vault_responder.lock().unwrap();

        assert_eq!(initiator.h, responder.h);

        let s1 = vault_in.secret_export(&initiator.encrypt_key).unwrap();
        let s2 = vault_re.secret_export(&responder.decrypt_key).unwrap();

        assert_eq!(s1, s2);

        let s1 = vault_in.secret_export(&initiator.decrypt_key).unwrap();
        let s2 = vault_re.secret_export(&responder.encrypt_key).unwrap();

        assert_eq!(s1, s2);
    }
}
