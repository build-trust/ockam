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
    _remote_static_public_key: Option<PublicKey>,
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
            _remote_static_public_key: None,
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
            self.identity_public_key = Some(self.vault.secret_public_key_get(ik)?);
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

        Ok(CompletedKeyExchange::new(h, encrypt_key, decrypt_key))
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
        self._remote_static_public_key = Some(rs);
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
        self._remote_static_public_key = Some(rs);
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
            "b8fddc038e170b1be4f54e193388043ef7282f691669d1907f31457c8be0497d";
        const INIT_EPH: &str = "208ed05ac237f19324fb97380d3d0be0caf8b13ad9558475d68f31a059e2e643";
        const RESP_STATIC: &str =
            "e8e007c2161efb952c6eb238e60d27e6ee8c329a6dbf6c45209746f774441e7a";
        const RESP_EPH: &str = "9061aafa5f178bd51a35f1b45634db3ab83bc743b5e21e9813388128ba7a4564";
        const MSG_1_CIPHERTEXT: &str =
            "03d5f778254408cdd461166d015da24f90827270ec6e5f445545c93059568060";
        const MSG_1_PAYLOAD: &str = "";
        const MSG_2_CIPHERTEXT: &str = "eb205a2af69bf453ad02333af1a78765dd7ceec86134e002fc9e24b47cb7783bb96f3c1b8a3faba26f3db59a7efcdbcd715fbee7951a4aa9a82d9bbf88dbf36a1f21d442c9282b0b24c5b7800cf67d55f93290cf564e32e8ceac4b4f9e0822c1";
        const MSG_2_PAYLOAD: &str = "";
        const MSG_3_CIPHERTEXT: &str = "43ab977d5be69779098ca1eea2124c71e1a02c9637dcdaa5d7f09c8bdd33b5844b4ec0fd51d439c65de3112a9bb7ad762504d7f8e4aaca071c791395a5b64d1a";
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
            "f03b71e5d8921855c3859dceb767efd64dddf25d8b3055f917061b40a21a494c";
        const RESP_STATIC: &str =
            "20bf730a9951f4a7c4f006bb8be759f16c1d0c66739f18ecf17dc755ed6ded51";
        const INIT_EPH: &str = "d864f25781681b9c50e32ebfdaa44a78ca7a6bf653df18e74df5ba5522a7507b";
        const RESP_EPH: &str = "985a88d0a30e161a757168d4d5fb6bce43ecfa152576d2db79a3ac316cb57e6a";
        const MSG_1_PAYLOAD: &str = "746573745f6d73675f30";
        const MSG_1_CIPHERTEXT: &str =
            "8f6450d283c75df98c3a69e5e90324ae1a4e244ed22aa7cd34c22f85825de635746573745f6d73675f30";
        const MSG_2_PAYLOAD: &str = "746573745f6d73675f31";
        const MSG_2_CIPHERTEXT: &str = "60537c177df729fc673e1a95382a49f33ba37655a64eeef8440e97121745345418242a6da2efdc7302d1aef36199327b841b3cb0ed5cfe1266b44f56446c36af3274fadac9f12d13d71720941205b9cfc58c56b9a4b1e443b85f61270fcea026a6f6e3aea01b92ca22d4";
        const MSG_3_PAYLOAD: &str = "746573745f6d73675f32";
        const MSG_3_CIPHERTEXT: &str = "43864de3c39719e8ea121c4382cd61877876229be549d4521b06b848656ee2a70ca53e3b4016d96830472f28c5a84bf7cc2e39e72a0d2c11318c48c577749fd6432f39a2c8f157723499";

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
            "f00dd4f8b97089ca8dcf1f3321fe7006785b59240168908eb4ee808d29eb2b44";
        const INIT_EPH: &str = "089b3b342fd868fd57bc26ba9b179396901de5afde41eefd00d326c8484ab45f";
        const RESP_STATIC: &str =
            "0078ff88eece9ddd05578cc7008cfa6230b444fb9314d522047bd81c25265555";
        const RESP_EPH: &str = "60f0e93c20a2ab370557b3ff35dd11023d0fe1d81f1063b50f81fe4e108ac05e";
        const MSG_1_CIPHERTEXT: &str =
            "5a302d4f1e1e7c74d089378973d4b3a4082d216ce8305957435178b0def80a7f";
        const MSG_1_PAYLOAD: &str = "";
        const MSG_2_CIPHERTEXT: &str = "4a2d1c6fb26620b8f35ad7991e429946ebf6fabfc723176b5b08b88ae2fbff05a0164ed7490899b680f95f3473426a3bf8de84bfc5e2fdb7ed295033df3779ce3c027388378bfc30770fb957c1e6d37839393f8d12e4ff59e9df46167ce5e746";
        const MSG_2_PAYLOAD: &str = "";
        const MSG_3_CIPHERTEXT: &str = "868a5886eb7ca265ff719c77f2c64364ff6633161309e4e134ef1266f61c28d854e2a6ebaf9d78e69c682f300b5a158f3348fb7bb64b6f27dfeab98dd0bce03a";
        const MSG_3_PAYLOAD: &str = "";

        let mut vault = VaultMutex::create(SoftwareVault::default());

        let initiator = mock_prologue(&mut vault, INIT_STATIC, INIT_EPH);
        let responder = mock_prologue(&mut vault, RESP_STATIC, RESP_EPH);

        let mut initiator = Initiator::new(initiator);
        let mut responder = Responder::new(responder);

        let res = initiator.generate_request(&decode(MSG_1_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(encode(&msg1), MSG_1_CIPHERTEXT);

        let res = responder.handle_response(&msg1);
        assert!(res.is_ok());
        let res = responder.generate_request(&decode(MSG_2_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(encode(&msg2), MSG_2_CIPHERTEXT);

        let res = initiator.handle_response(&msg2);
        assert!(res.is_ok());
        let res = initiator.generate_request(&decode(MSG_3_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(encode(&msg3), MSG_3_CIPHERTEXT);

        let res = responder.handle_response(&msg3);
        assert!(res.is_ok());

        let res = initiator.finalize();
        assert!(res.is_ok());
        let alice = res.unwrap();
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
            _remote_static_public_key: None,
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
