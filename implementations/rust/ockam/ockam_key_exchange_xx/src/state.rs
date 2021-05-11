use crate::{XXError, XXVault, AES_GCM_TAGSIZE, SHA256_SIZE};
use ockam_core::Result;
use ockam_key_exchange_core::CompletedKeyExchange;
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType, AES256_SECRET_LENGTH,
    CURVE25519_PUBLIC_LENGTH, CURVE25519_SECRET_LENGTH,
};
use zeroize::Zeroize;

mod dh_state;
pub(crate) use dh_state::*;

/// Represents the XX Handshake
pub(crate) struct State<V: XXVault> {
    run_prologue: bool,
    identity_key: Option<Secret>,
    identity_public_key: Option<PublicKey>,
    ephemeral_secret: Option<Secret>,
    ephemeral_public: Option<PublicKey>,
    remote_static_public_key: Option<PublicKey>,
    remote_ephemeral_public_key: Option<PublicKey>,
    dh_state: DhState<V>,
    nonce: u16,
    h: Option<[u8; SHA256_SIZE]>,
    vault: V,
}

impl<V: XXVault> Zeroize for State<V> {
    fn zeroize(&mut self) {
        self.nonce.zeroize();
        self.h.zeroize()
    }
}

impl<V: XXVault> std::fmt::Debug for State<V> {
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

impl<V: XXVault> State<V> {
    pub(crate) fn new(vault: &V) -> Result<Self> {
        Ok(Self {
            run_prologue: true,
            identity_key: None,
            identity_public_key: None,
            ephemeral_secret: None,
            ephemeral_public: None,
            remote_static_public_key: None,
            remote_ephemeral_public_key: None,
            dh_state: DhState::empty(vault.clone()),
            nonce: 0,
            h: None,
            vault: vault.clone(),
        })
    }
}

impl<V: XXVault> State<V> {
    fn get_symmetric_key_type_and_length(&self) -> (SecretType, usize) {
        (SecretType::Aes, AES256_SECRET_LENGTH)
    }

    fn get_protocol_name(&self) -> &'static [u8] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> Result<()> {
        let attributes = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        // 1. Generate a static key pair for this handshake and set it to `s`
        if let Some(ik) = &self.identity_key {
            self.identity_public_key = Some(self.vault.secret_public_key_get(&ik)?);
        } else {
            let static_secret_handle = self.vault.secret_generate(attributes)?;
            self.identity_public_key =
                Some(self.vault.secret_public_key_get(&static_secret_handle)?);
            self.identity_key = Some(static_secret_handle)
        };

        // 2. Generate an ephemeral key pair for this handshake and set it to e
        let ephemeral_secret_handle = self.vault.secret_generate(attributes)?;
        self.ephemeral_public = Some(self.vault.secret_public_key_get(&ephemeral_secret_handle)?);
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
        self.dh_state = DhState::new(&h, self.vault.clone())?;
        self.h = Some(self.vault.sha256(&h)?);

        Ok(())
    }

    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<[u8; 32]> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut input = h.to_vec();
        input.extend_from_slice(data.as_ref());
        let h = self.vault.sha256(&input)?;
        Ok(h)
    }

    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        plaintext: B,
    ) -> Result<(Vec<u8>, [u8; 32])> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());

        let ciphertext_and_tag = {
            let key = self.dh_state.key().ok_or(XXError::InvalidState)?;
            self.vault
                .aead_aes_gcm_encrypt(key, plaintext.as_ref(), nonce.as_ref(), h)?
        };
        let h = self.mix_hash(&ciphertext_and_tag)?;
        Ok((ciphertext_and_tag, h))
    }

    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> Result<(Vec<u8>, [u8; 32])> {
        let h = &self.h.ok_or(XXError::InvalidState)?;

        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = {
            let key = self.dh_state.key().ok_or(XXError::InvalidState)?;
            self.vault
                .aead_aes_gcm_decrypt(key, ciphertext, nonce.as_ref(), h)?
        };
        let h = self.mix_hash(ciphertext)?;
        Ok((plaintext, h))
    }

    /// Split step in Noise protocol
    fn split(&mut self) -> Result<(Secret, Secret)> {
        let ck = self.dh_state.ck().ok_or(XXError::InvalidState)?;

        let symmetric_key_info = self.get_symmetric_key_type_and_length();
        let attributes = SecretAttributes::new(
            symmetric_key_info.0,
            SecretPersistence::Ephemeral,
            symmetric_key_info.1,
        );
        let mut hkdf_output =
            self.vault
                .hkdf_sha256(ck, b"", None, vec![attributes, attributes])?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let res1 = hkdf_output.pop().unwrap();
        let res0 = hkdf_output.pop().unwrap();

        Ok((res0, res1))
    }

    /// Set this state up to send and receive messages
    fn finalize(self, encrypt_key: Secret, decrypt_key: Secret) -> Result<CompletedKeyExchange> {
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

impl<V: XXVault> State<V> {
    pub(crate) fn run_prologue(&mut self) -> Result<()> {
        if self.run_prologue {
            self.prologue()
        } else {
            Ok(())
        }
    }
}

impl<V: XXVault> State<V> {
    /// Encode the first message to be sent
    pub(crate) fn encode_message_1<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>> {
        let ephemeral_public_key = self
            .ephemeral_public
            .as_ref()
            .ok_or(XXError::InvalidState)?
            .clone();

        let payload = payload.as_ref();
        self.h = Some(self.mix_hash(ephemeral_public_key.as_ref())?);
        self.h = Some(self.mix_hash(payload)?);

        let mut output = ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub(crate) fn decode_message_2<B: AsRef<[u8]>>(&mut self, message: B) -> Result<Vec<u8>> {
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

        self.h = Some(self.mix_hash(re.as_ref())?);
        self.dh_state.dh(&ephemeral_secret_handle, &re)?;
        self.remote_ephemeral_public_key = Some(re);
        let (rs, h) = self.decrypt_and_mix_hash(encrypted_rs_and_tag)?;
        self.h = Some(h);
        let rs = PublicKey::new(rs);
        self.dh_state.dh(&ephemeral_secret_handle, &rs)?;
        self.remote_static_public_key = Some(rs);
        self.nonce = 0;

        let (payload, h) = self.decrypt_and_mix_hash(encrypted_payload_and_tag)?;
        self.h = Some(h);
        self.nonce += 1;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub(crate) fn encode_message_3<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>> {
        let static_secret = self.identity_key.clone().ok_or(XXError::InvalidState)?;

        let static_public = self
            .identity_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let remote_ephemeral_public_key = self
            .remote_ephemeral_public_key
            .clone()
            .ok_or(XXError::InvalidState)?;

        let (mut encrypted_s_and_tag, h) = self.encrypt_and_mix_hash(static_public.as_ref())?;
        self.h = Some(h);
        self.dh_state
            .dh(&static_secret, &remote_ephemeral_public_key)?;
        self.nonce = 0;
        let (mut encrypted_payload_and_tag, h) = self.encrypt_and_mix_hash(payload)?;
        self.h = Some(h);
        self.nonce += 1;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    pub(crate) fn finalize_initiator(mut self) -> Result<CompletedKeyExchange> {
        let keys = { self.split()? };

        self.finalize(keys.1, keys.0)
    }
}

impl<V: XXVault> State<V> {
    /// Decode the first message sent
    pub(crate) fn decode_message_1<B: AsRef<[u8]>>(&mut self, message_1: B) -> Result<Vec<u8>> {
        let public_key_size = CURVE25519_PUBLIC_LENGTH;
        let message_1 = message_1.as_ref();
        if message_1.len() < public_key_size {
            return Err(XXError::MessageLenMismatch.into());
        }

        let re = &message_1[..public_key_size];
        let re = PublicKey::new(re.to_vec());
        self.h = Some(self.mix_hash(re.as_ref())?);
        self.h = Some(self.mix_hash(&message_1[public_key_size..])?);
        self.remote_ephemeral_public_key = Some(re);
        Ok(message_1[public_key_size..].to_vec())
    }

    /// Encode the second message to be sent
    pub(crate) fn encode_message_2<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>> {
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

        self.h = Some(self.mix_hash(ephemeral_public.as_ref())?);
        self.dh_state
            .dh(&ephemeral_secret, &remote_ephemeral_public_key)?;

        let (mut encrypted_s_and_tag, h) = self.encrypt_and_mix_hash(static_public.as_ref())?;
        self.h = Some(h);
        self.dh_state
            .dh(&static_secret, &remote_ephemeral_public_key)?;
        self.nonce = 0;
        let (mut encrypted_payload_and_tag, h) = self.encrypt_and_mix_hash(payload)?;
        self.h = Some(h);
        self.nonce += 1;

        let mut output = ephemeral_public.as_ref().to_vec();
        output.append(&mut encrypted_s_and_tag);
        output.append(&mut encrypted_payload_and_tag);
        Ok(output)
    }

    /// Decode the final message received for the handshake
    pub(crate) fn decode_message_3<B: AsRef<[u8]>>(&mut self, message_3: B) -> Result<Vec<u8>> {
        let public_key_size = CURVE25519_PUBLIC_LENGTH;
        let message_3 = message_3.as_ref();
        if message_3.len() < public_key_size + AES_GCM_TAGSIZE {
            return Err(XXError::MessageLenMismatch.into());
        }

        let ephemeral_secret = &self.ephemeral_secret.clone().ok_or(XXError::InvalidState)?;

        let (rs, h) = self.decrypt_and_mix_hash(&message_3[..public_key_size + AES_GCM_TAGSIZE])?;
        self.h = Some(h);
        let rs = PublicKey::new(rs);
        self.dh_state.dh(ephemeral_secret, &rs)?;
        self.nonce = 0;
        let (payload, h) =
            self.decrypt_and_mix_hash(&message_3[public_key_size + AES_GCM_TAGSIZE..])?;
        self.h = Some(h);
        self.nonce += 1;
        self.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    pub(crate) fn finalize_responder(mut self) -> Result<CompletedKeyExchange> {
        let keys = { self.split()? };

        self.finalize(keys.0, keys.1)
    }
}

#[cfg(test)]
mod tests {
    use crate::state::{DhState, State};
    use crate::{Initiator, Responder, XXVault};
    use ockam_core::hex::{decode, encode};
    use ockam_key_exchange_core::KeyExchanger;
    use ockam_vault::SoftwareVault;
    use ockam_vault_core::{
        SecretAttributes, SecretPersistence, SecretType, SecretVault, SymmetricVault,
        CURVE25519_SECRET_LENGTH,
    };
    use ockam_vault_sync_core::VaultMutex;

    #[test]
    fn prologue() {
        let mut vault = VaultMutex::create(SoftwareVault::default());

        let exp_h = [
            93, 247, 43, 103, 185, 101, 173, 209, 22, 143, 10, 108, 117, 109, 242, 28, 32, 79, 126,
            100, 252, 104, 43, 230, 163, 171, 75, 104, 44, 141, 182, 75,
        ];

        let mut state = State::new(&vault).unwrap();
        let res = state.prologue();
        assert!(res.is_ok());
        assert_eq!(state.h.unwrap(), exp_h);

        let ck = vault.secret_export(&state.dh_state.ck.unwrap()).unwrap();

        assert_eq!(ck.as_ref(), *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0");
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

    fn mock_handshake(
        init_static: &'static str,
        init_eph: &'static str,
        resp_static: &'static str,
        resp_eph: &'static str,
        msg_1_payload: &'static str,
        msg_1_ciphertext: &'static str,
        msg_2_payload: &'static str,
        msg_2_ciphertext: &'static str,
        msg_3_payload: &'static str,
        msg_3_ciphertext: &'static str,
    ) {
        let mut vault = VaultMutex::create(SoftwareVault::default());

        let mut initiator = mock_prologue(&mut vault, init_static, init_eph);
        let mut responder = mock_prologue(&mut vault, resp_static, resp_eph);

        let res = initiator.encode_message_1(decode(msg_1_payload).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(encode(&msg1), msg_1_ciphertext);

        let res = responder.decode_message_1(msg1);
        assert!(res.is_ok());

        let res = responder.encode_message_2(decode(msg_2_payload).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(encode(&msg2), msg_2_ciphertext);

        let res = initiator.decode_message_2(msg2);
        assert!(res.is_ok());
        let res = initiator.encode_message_3(decode(msg_3_payload).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(encode(&msg3), msg_3_ciphertext);

        let res = responder.decode_message_3(msg3);
        assert!(res.is_ok());

        let res = initiator.finalize_initiator();
        assert!(res.is_ok());
        let res = responder.finalize_responder();
        assert!(res.is_ok());
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

        let mut vault = VaultMutex::create(SoftwareVault::default());

        let initiator = mock_prologue(&mut vault, INIT_STATIC, INIT_EPH);
        let responder = mock_prologue(&mut vault, RESP_STATIC, RESP_EPH);

        let mut initiator = Initiator::new(initiator);
        let mut responder = Responder::new(responder);

        let res = responder.process(&[]);
        assert!(res.is_err());
        let res = initiator.process(&decode(MSG_1_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(encode(&msg1), MSG_1_CIPHERTEXT);

        let res = responder.process(&msg1);
        assert!(res.is_ok());
        let res = responder.process(&decode(MSG_2_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(encode(&msg2), MSG_2_CIPHERTEXT);

        let res = initiator.process(&msg2);
        assert!(res.is_ok());
        let res = initiator.process(&decode(MSG_3_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(encode(&msg3), MSG_3_CIPHERTEXT);

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
        assert_eq!(alice.h(), bob.h());
        let res =
            vault.aead_aes_gcm_encrypt(alice.encrypt_key(), b"hello bob", &[0u8; 12], alice.h());

        assert!(res.is_ok());
        let ciphertext = res.unwrap();

        let res = vault.aead_aes_gcm_decrypt(bob.decrypt_key(), &ciphertext, &[0u8; 12], bob.h());
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, b"hello bob");

        let res =
            vault.aead_aes_gcm_encrypt(bob.encrypt_key(), b"hello alice", &[1u8; 12], bob.h());
        assert!(res.is_ok());
        let ciphertext = res.unwrap();
        let res =
            vault.aead_aes_gcm_decrypt(alice.decrypt_key(), &ciphertext, &[1u8; 12], alice.h());
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, b"hello alice");
    }

    fn mock_prologue<V: XXVault>(
        vault: &mut V,
        static_private: &str,
        ephemeral_private: &str,
    ) -> State<V> {
        let attributes = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        // Static x25519 for this handshake, `s`
        let static_secret_handle = vault
            .secret_import(&decode(static_private).unwrap(), attributes)
            .unwrap();
        let static_public_key = vault.secret_public_key_get(&static_secret_handle).unwrap();

        // Ephemeral x25519 for this handshake, `e`
        let ephemeral_secret_handle = vault
            .secret_import(&decode(ephemeral_private).unwrap(), attributes)
            .unwrap();
        let ephemeral_public_key = vault
            .secret_public_key_get(&ephemeral_secret_handle)
            .unwrap();

        let h = vault
            .sha256(b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0")
            .unwrap();
        let ck = *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0";

        let attributes =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, ck.len());
        let ck = vault.secret_import(&ck[..], attributes).unwrap();

        State {
            run_prologue: false,
            identity_key: Some(static_secret_handle),
            identity_public_key: Some(static_public_key),
            ephemeral_secret: Some(ephemeral_secret_handle),
            ephemeral_public: Some(ephemeral_public_key),
            remote_static_public_key: None,
            remote_ephemeral_public_key: None,
            dh_state: DhState {
                key: None,
                ck: Some(ck),
                vault: vault.clone(),
            },
            nonce: 0,
            h: Some(h),
            vault: vault.clone(),
        }
    }
}
