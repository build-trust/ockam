use crate::{XXError, XXVault, AES_GCM_TAGSIZE, SHA256_SIZE};
use ockam_key_exchange_core::CompletedKeyExchange;
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_PUBLIC_LENGTH, CURVE25519_SECRET_LENGTH,
};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;

struct DhState {
    key: Option<Secret>,
    ck: Option<Secret>,
}

impl DhState {
    fn empty() -> Self {
        Self {
            key: None,
            ck: None,
        }
    }

    fn new(protocol_name: &[u8; 32], vault: &mut dyn XXVault) -> ockam_core::Result<Self> {
        let attributes = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE,
        );

        let ck = vault.secret_import(protocol_name, attributes)?;

        Ok(Self {
            key: None,
            ck: Some(ck),
        })
    }
}

impl DhState {
    pub fn key(&self) -> Option<&Secret> {
        self.key.as_ref()
    }
    pub fn ck(&self) -> Option<&Secret> {
        self.ck.as_ref()
    }
}

impl DhState {
    fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }
    /// Perform the diffie-hellman computation
    fn dh(
        &mut self,
        secret_handle: &Secret,
        public_key: &[u8],
        vault: &mut dyn XXVault,
    ) -> ockam_core::Result<()> {
        let ck = self.ck.as_ref().ok_or(XXError::InvalidState)?;

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
            vault.hkdf_sha256(ck, b"", Some(&ecdh), vec![attributes_ck, attributes_k])?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let key = self.key.take();
        if key.is_some() {
            vault.secret_destroy(key.unwrap())?;
        }

        self.key = Some(hkdf_output.pop().unwrap());

        let ck = self.ck.take();

        vault.secret_destroy(ck.unwrap())?;
        self.ck = Some(hkdf_output.pop().unwrap());

        Ok(())
    }
}

/// Represents the XX Handshake
pub(crate) struct State {
    identity_key: Option<Secret>,
    identity_public_key: Option<PublicKey>,
    ephemeral_secret: Option<Secret>,
    ephemeral_public: Option<PublicKey>,
    remote_static_public_key: Option<PublicKey>,
    remote_ephemeral_public_key: Option<PublicKey>,
    dh_state: DhState,
    nonce: u16,
    h: Option<[u8; SHA256_SIZE]>,
    vault: Arc<Mutex<dyn XXVault>>,
}

impl Zeroize for State {
    fn zeroize(&mut self) {
        self.nonce.zeroize();
        self.h.zeroize()
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "SymmetricState {{ key: {:?}, nonce: {:?}, h: {:?}, ck: {:?} }}",
            self.dh_state.key(),
            self.nonce,
            self.h,
            self.dh_state.ck()
        )
    }
}

impl State {
    pub(crate) fn new(vault: Arc<Mutex<dyn XXVault>>) -> Self {
        Self {
            identity_key: None,
            identity_public_key: None,
            ephemeral_secret: None,
            ephemeral_public: None,
            remote_static_public_key: None,
            remote_ephemeral_public_key: None,
            dh_state: DhState::empty(),
            nonce: 0,
            h: None,
            vault,
        }
    }
}

impl State {
    fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }

    fn get_protocol_name(&self) -> &'static [u8] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    /// Create a new `HandshakeState` starting with the prologue
    pub(crate) fn prologue(&mut self) -> ockam_core::Result<()> {
        let attributes = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        // 1. Generate a static key pair for this handshake and set it to `s`
        let mut vault = self.vault.lock().unwrap();
        if let Some(ik) = &self.identity_key {
            self.identity_public_key = Some(vault.secret_public_key_get(&ik)?);
        } else {
            let static_secret_handle = vault.secret_generate(attributes)?;
            self.identity_public_key = Some(vault.secret_public_key_get(&static_secret_handle)?);
            self.identity_key = Some(static_secret_handle)
        };

        // 2. Generate an ephemeral key pair for this handshake and set it to e
        let ephemeral_secret_handle = vault.secret_generate(attributes)?;
        self.ephemeral_public = Some(vault.secret_public_key_get(&ephemeral_secret_handle)?);
        self.ephemeral_secret = Some(ephemeral_secret_handle);

        // 3. Set k to empty, Set n to 0
        // let nonce = 0;
        self.nonce = 0;

        // 4. Set h and ck to protocol name
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let mut h = [0u8; SHA256_SIZE];
        h[..self.get_protocol_name().len()].copy_from_slice(self.get_protocol_name());
        self.dh_state = DhState::new(&h, vault.deref_mut())?;
        self.h = Some(h);

        Ok(())
    }

    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(
        &self,
        data: B,
        vault: &dyn XXVault,
    ) -> ockam_core::Result<[u8; 32]> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut input = h.to_vec();
        input.extend_from_slice(data.as_ref());
        let h = vault.sha256(&input)?;
        Ok(h)
    }

    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &self,
        plaintext: B,
        vault: &mut dyn XXVault,
    ) -> ockam_core::Result<(Vec<u8>, [u8; 32])> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());

        let ciphertext_and_tag = {
            let key = self.dh_state.key().ok_or(XXError::InvalidState)?;
            vault.aead_aes_gcm_encrypt(key, plaintext.as_ref(), nonce.as_ref(), h)?
        };
        let h = self.mix_hash(&ciphertext_and_tag, vault)?;
        Ok((ciphertext_and_tag, h))
    }

    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &self,
        ciphertext: B,
        vault: &mut dyn XXVault,
    ) -> ockam_core::Result<(Vec<u8>, [u8; 32])> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = {
            let key = self.dh_state.key().ok_or(XXError::InvalidState)?;
            vault.aead_aes_gcm_decrypt(key, ciphertext, nonce.as_ref(), h)?
        };
        let h = self.mix_hash(ciphertext, vault)?;
        Ok((plaintext, h))
    }

    /// Split step in Noise protocol
    fn split(&self, vault: &mut dyn XXVault) -> ockam_core::Result<(Secret, Secret)> {
        let ck = self.dh_state.ck().ok_or(XXError::InvalidState)?;

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

        Ok(CompletedKeyExchange::new(
            h,
            encrypt_key,
            decrypt_key,
            local_static_secret,
            remote_static_public_key,
        ))
    }
}

impl State {
    /// Encode the first message to be sent
    pub(crate) fn encode_message_1<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let ephemeral_public_key = self
            .ephemeral_public
            .as_ref()
            .ok_or(XXError::InvalidState)?
            .clone();

        let payload = payload.as_ref();
        let vault = self.vault.lock().unwrap();
        self.h = Some(self.mix_hash(ephemeral_public_key.as_ref(), vault.deref())?);
        self.h = Some(self.mix_hash(payload, vault.deref())?);

        let mut output = ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub(crate) fn decode_message_2<B: AsRef<[u8]>>(
        &mut self,
        message: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let public_key_size = CURVE25519_PUBLIC_LENGTH;
        let message = message.as_ref();
        if message.len() < 2 * public_key_size + AES_GCM_TAGSIZE {
            return Err(XXError::MessageLenMismatch.into());
        }

        let ephemeral_secret_handle = self.ephemeral_secret.clone().ok_or(XXError::InvalidState)?;

        let mut index_l = 0;
        let mut index_r = public_key_size;
        let re = &message[..index_r];
        let re = PublicKey::new(re.to_vec());
        index_l += public_key_size;
        index_r += public_key_size + AES_GCM_TAGSIZE;
        let encrypted_rs_and_tag = &message[index_l..index_r];
        let encrypted_payload_and_tag = &message[index_r..];

        let mut vault = self.vault.lock().unwrap();
        self.h = Some(self.mix_hash(re.as_ref(), vault.deref())?);
        self.dh_state
            .dh(&ephemeral_secret_handle, re.as_ref(), vault.deref_mut())?;
        self.remote_ephemeral_public_key = Some(re);
        let (rs, h) = self.decrypt_and_mix_hash(encrypted_rs_and_tag, vault.deref_mut())?;
        self.h = Some(h);
        let rs = PublicKey::new(rs);
        self.dh_state
            .dh(&ephemeral_secret_handle, rs.as_ref(), vault.deref_mut())?;
        self.remote_static_public_key = Some(rs);
        self.nonce = 0;

        let (payload, h) =
            self.decrypt_and_mix_hash(encrypted_payload_and_tag, vault.deref_mut())?;
        self.h = Some(h);
        self.nonce += 1;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub(crate) fn encode_message_3<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let static_secret = self.identity_key.clone().ok_or(XXError::InvalidState)?;

        let static_public = self
            .identity_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let remote_ephemeral_public_key = self
            .remote_ephemeral_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let mut vault = self.vault.lock().unwrap();
        let (mut encrypted_s_and_tag, h) =
            self.encrypt_and_mix_hash(static_public.as_ref(), vault.deref_mut())?;
        self.h = Some(h);
        self.dh_state.dh(
            &static_secret,
            remote_ephemeral_public_key.as_ref(),
            vault.deref_mut(),
        )?;
        self.nonce = 0;
        let (mut encrypted_payload_and_tag, h) =
            self.encrypt_and_mix_hash(payload, vault.deref_mut())?;
        self.h = Some(h);
        self.nonce += 1;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    pub(crate) fn finalize_initiator(self) -> ockam_core::Result<CompletedKeyExchange> {
        let keys = {
            let mut vault = self.vault.lock().unwrap();
            self.split(vault.deref_mut())?
        };

        self.finalize(keys.1, keys.0)
    }
}

impl State {
    /// Decode the first message sent
    pub(crate) fn decode_message_1<B: AsRef<[u8]>>(
        &mut self,
        message_1: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let public_key_size = CURVE25519_PUBLIC_LENGTH;
        let message_1 = message_1.as_ref();
        if message_1.len() < public_key_size {
            return Err(XXError::MessageLenMismatch.into());
        }

        let re = &message_1[..public_key_size];
        let re = PublicKey::new(re.to_vec());
        let vault = self.vault.lock().unwrap();
        self.h = Some(self.mix_hash(re.as_ref(), vault.deref())?);
        self.h = Some(self.mix_hash(&message_1[public_key_size..], vault.deref())?);
        self.remote_ephemeral_public_key = Some(re);
        Ok(message_1[public_key_size..].to_vec())
    }

    /// Encode the second message to be sent
    pub(crate) fn encode_message_2<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let static_secret = self.identity_key.clone().ok_or(XXError::InvalidState)?;
        let static_public = self
            .identity_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;
        let ephemeral_public = self.ephemeral_public.clone().ok_or(XXError::InvalidState)?;
        let ephemeral_secret = self.ephemeral_secret.clone().ok_or(XXError::InvalidState)?;
        let remote_ephemeral_public_key = self
            .remote_ephemeral_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let mut vault = self.vault.lock().unwrap();
        self.h = Some(self.mix_hash(ephemeral_public.as_ref(), vault.deref_mut())?);
        self.dh_state.dh(
            &ephemeral_secret,
            remote_ephemeral_public_key.as_ref(),
            vault.deref_mut(),
        )?;

        let (mut encrypted_s_and_tag, h) =
            self.encrypt_and_mix_hash(static_public.as_ref(), vault.deref_mut())?;
        self.h = Some(h);
        self.dh_state.dh(
            &static_secret,
            remote_ephemeral_public_key.as_ref(),
            vault.deref_mut(),
        )?;
        self.nonce = 0;
        let (mut encrypted_payload_and_tag, h) =
            self.encrypt_and_mix_hash(payload, vault.deref_mut())?;
        self.h = Some(h);
        self.nonce += 1;

        let mut output = ephemeral_public.as_ref().to_vec();
        output.append(&mut encrypted_s_and_tag);
        output.append(&mut encrypted_payload_and_tag);
        Ok(output)
    }

    /// Decode the final message received for the handshake
    pub(crate) fn decode_message_3<B: AsRef<[u8]>>(
        &mut self,
        message_3: B,
    ) -> ockam_core::Result<Vec<u8>> {
        let public_key_size = CURVE25519_PUBLIC_LENGTH;
        let message_3 = message_3.as_ref();
        if message_3.len() < public_key_size + AES_GCM_TAGSIZE {
            return Err(XXError::MessageLenMismatch.into());
        }

        let ephemeral_secret = &self.ephemeral_secret.clone().ok_or(XXError::InvalidState)?;

        let mut vault = self.vault.lock().unwrap();
        let (rs, h) = self.decrypt_and_mix_hash(
            &message_3[..public_key_size + AES_GCM_TAGSIZE],
            vault.deref_mut(),
        )?;
        self.h = Some(h);
        let rs = PublicKey::new(rs);
        self.dh_state
            .dh(ephemeral_secret, rs.as_ref(), vault.deref_mut())?;
        self.nonce = 0;
        let (payload, h) = self.decrypt_and_mix_hash(
            &message_3[public_key_size + AES_GCM_TAGSIZE..],
            vault.deref_mut(),
        )?;
        self.h = Some(h);
        self.nonce += 1;
        self.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    pub(crate) fn finalize_responder(self) -> ockam_core::Result<CompletedKeyExchange> {
        let keys = {
            let mut vault = self.vault.lock().unwrap();
            self.split(vault.deref_mut())?
        };

        self.finalize(keys.0, keys.1)
    }
}
