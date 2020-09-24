use super::{
    CompletedKeyExchange, HandshakeStateData, KeyExchange, KeyExchanger, SymmetricStateData,
    AES256_KEYSIZE, SHA256_SIZE,
};
use crate::error::KexExchangeFailError;
use ockam_vault::{
    error::{VaultFailError, VaultFailErrorKind},
    types::{
        PublicKey, SecretKey, SecretKeyAttributes, SecretKeyContext, SecretKeyType,
        SecretPersistenceType, SecretPurposeType,
    },
    Vault,
};

/// Represents the XX Handshake
#[derive(Debug)]
struct SymmetricState<'a, V: Vault> {
    handshake: HandshakeStateData,
    key: Option<SecretKeyContext>,
    nonce: u16,
    state: SymmetricStateData,
    vault: &'a mut V,
}

impl<'a, V: Vault> SymmetricState<'a, V> {
    pub fn new(vault: &'a mut V) -> Self {
        Self {
            handshake: HandshakeStateData {
                ephemeral_public_key: PublicKey::Curve25519([0u8; 32]),
                ephemeral_secret_handle: SecretKeyContext::Memory(0),
                static_public_key: PublicKey::Curve25519([0u8; 32]),
                static_secret_handle: SecretKeyContext::Memory(0),
                remote_static_public_key: None,
                remote_ephemeral_public_key: None,
            },
            key: None,
            nonce: 0,
            state: SymmetricStateData {
                h: [0u8; 32],
                ck: [0u8; 32],
            },
            vault,
        }
    }
}

impl<'a, V: Vault> KeyExchange for SymmetricState<'a, V> {
    const CSUITE: &'static [u8] = b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0";

    /// Create a new `HandshakeState` starting with the prologue
    fn prologue(&mut self) -> Result<(), VaultFailError> {
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Persistent,
        };
        // 1. Generate a static 25519 keypair for this handshake and set it to `s`
        let static_secret_handle = self.vault.secret_generate(attributes)?;
        let static_public_key = self.vault.secret_public_key_get(static_secret_handle)?;

        attributes.persistence = SecretPersistenceType::Ephemeral;
        // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
        let ephemeral_secret_handle = self.vault.secret_generate(attributes)?;
        let ephemeral_public_key = self.vault.secret_public_key_get(ephemeral_secret_handle)?;

        // 3. Set k to empty, Set n to 0
        // let nonce = 0;

        // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let mut h = [0u8; SHA256_SIZE];
        h[..Self::CSUITE.len()].copy_from_slice(Self::CSUITE);
        let ck = h;
        let h = self.vault.sha256(h)?;

        self.handshake = HandshakeStateData {
            static_public_key,
            static_secret_handle,
            ephemeral_public_key,
            ephemeral_secret_handle,
            remote_ephemeral_public_key: None,
            remote_static_public_key: None,
        };
        self.key = None;
        self.nonce = 0;
        self.state = SymmetricStateData { h, ck };
        Ok(())
    }

    /// Perform the diffie-hellman computation
    fn dh(
        &mut self,
        secret_handle: SecretKeyContext,
        public_key: PublicKey,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.vault.ec_diffie_hellman_hkdf_sha256(
            secret_handle,
            public_key,
            self.state.ck.as_ref(),
            SHA256_SIZE + AES256_KEYSIZE,
        )
    }

    /// mix key step in Noise protocol
    fn mix_key<B: AsRef<[u8]>>(&mut self, hash: B) -> Result<(), VaultFailError> {
        let hash = hash.as_ref();
        self.state.ck.copy_from_slice(&hash[..SHA256_SIZE]);
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Aes256,
            persistence: SecretPersistenceType::Ephemeral,
            purpose: SecretPurposeType::KeyAgreement,
        };
        let key = SecretKey::Aes256(*array_ref![hash, SHA256_SIZE, AES256_KEYSIZE]);
        if self.key.is_some() {
            self.vault.secret_destroy(*self.key.as_ref().unwrap())?;
        }
        self.key = Some(self.vault.secret_import(&key, attributes)?);
        self.nonce = 0;
        Ok(())
    }

    /// mix hash step in Noise protocol
    fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError> {
        let mut input = self.state.h.to_vec();
        input.extend_from_slice(data.as_ref());
        self.state.h = self.vault.sha256(&input)?;
        Ok(())
    }

    /// Encrypt and mix step in Noise protocol
    fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        plaintext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext_and_tag = self.vault.aead_aes_gcm_encrypt(
            self.key.ok_or(VaultFailErrorKind::AeadAesGcmEncrypt)?,
            plaintext,
            nonce.as_ref(),
            &self.state.h,
        )?;
        self.mix_hash(&ciphertext_and_tag)?;
        self.nonce += 1;
        Ok(ciphertext_and_tag)
    }

    /// Decrypt and mix step in Noise protocol
    fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = self.vault.aead_aes_gcm_decrypt(
            self.key.ok_or(VaultFailErrorKind::AeadAesGcmDecrypt)?,
            ciphertext,
            nonce.as_ref(),
            &self.state.h,
        )?;
        self.mix_hash(ciphertext)?;
        self.nonce += 1;
        Ok(plaintext)
    }

    /// Split step in Noise protocol
    fn split(&mut self) -> Result<Vec<u8>, VaultFailError> {
        self.vault
            .hkdf_sha256(self.state.ck.as_ref(), &[], AES256_KEYSIZE + AES256_KEYSIZE)
    }

    /// Set this state up to send and receive messages
    fn finalize<B: AsRef<[u8]>, C: AsRef<[u8]>>(
        &mut self,
        encrypt_ref: B,
        decrypt_ref: C,
    ) -> Result<CompletedKeyExchange, VaultFailError> {
        let encrypt_ref = encrypt_ref.as_ref();
        let decrypt_ref = decrypt_ref.as_ref();
        debug_assert_eq!(encrypt_ref.len(), AES256_KEYSIZE);
        debug_assert_eq!(decrypt_ref.len(), AES256_KEYSIZE);
        let mut decrypt = [0u8; AES256_KEYSIZE];
        let mut encrypt = [0u8; AES256_KEYSIZE];
        decrypt.copy_from_slice(encrypt_ref);
        encrypt.copy_from_slice(decrypt_ref);
        let decrypt = SecretKey::Aes256(decrypt);
        let encrypt = SecretKey::Aes256(encrypt);
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Aes256,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        let decrypt_key = self.vault.secret_import(&decrypt, attributes)?;
        let encrypt_key = self.vault.secret_import(&encrypt, attributes)?;
        Ok(CompletedKeyExchange {
            h: self.state.h,
            encrypt_key,
            decrypt_key,
            local_static_secret: self.handshake.static_secret_handle,
            remote_static_public_key: self.handshake.remote_static_public_key.unwrap(),
        })
    }
}

/// Provides methods for handling the initiator role
#[derive(Debug)]
struct Initiator<'a, V: Vault>(SymmetricState<'a, V>);

impl<'a, V: Vault> Initiator<'a, V> {
    /// Encode the first message to be sent
    pub fn encode_message_1<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let payload = payload.as_ref();
        self.0.mix_hash(self.0.handshake.ephemeral_public_key)?;
        self.0.mix_hash(payload)?;

        let mut output = self.0.handshake.ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub fn decode_message_2<B: AsRef<[u8]>>(
        &mut self,
        message: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let message = message.as_ref();
        let mut re = [0u8; 32];
        re.copy_from_slice(&message[..32]);
        let encrypted_rs_and_tag = &message[32..80];
        let encrypted_payload_and_tag = &message[80..];

        let re = PublicKey::Curve25519(re);
        self.0.handshake.remote_ephemeral_public_key = Some(re);

        self.0.mix_hash(&re)?;
        let hash = self.0.dh(self.0.handshake.ephemeral_secret_handle, re)?;
        self.0.mix_key(&hash)?;
        let rs = self.0.decrypt_and_mix_hash(encrypted_rs_and_tag)?;
        let rs = PublicKey::Curve25519(*array_ref![rs, 0, 32]);
        self.0.handshake.remote_static_public_key = Some(rs);
        let hash = self.0.dh(self.0.handshake.ephemeral_secret_handle, rs)?;
        self.0.mix_key(&hash)?;
        let payload = self.0.decrypt_and_mix_hash(encrypted_payload_and_tag)?;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub fn encode_message_3<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut encrypted_s_and_tag = self
            .0
            .encrypt_and_mix_hash(self.0.handshake.static_public_key)?;
        let hash = self.0.dh(
            self.0.handshake.static_secret_handle,
            *self
                .0
                .handshake
                .remote_ephemeral_public_key
                .as_ref()
                .unwrap(),
        )?;
        self.0.mix_key(&hash)?;
        let mut encrypted_payload_and_tag = self.0.encrypt_and_mix_hash(payload)?;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    /// Setup this initiator to send and receive messages
    /// after encoding message 3
    pub fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        let keys = self.0.split()?;
        self.0
            .finalize(&keys[AES256_KEYSIZE..], &keys[..AES256_KEYSIZE])
    }
}

/// Provides methods for handling the responder role
#[derive(Debug)]
struct Responder<'a, V: Vault>(SymmetricState<'a, V>);

impl<'a, V: Vault> Responder<'a, V> {
    /// Decode the first message sent
    pub fn decode_message_1<B: AsRef<[u8]>>(
        &mut self,
        message_1: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let message_1 = message_1.as_ref();
        if message_1.len() < 32 {
            return Err(VaultFailErrorKind::SecretSizeMismatch.into());
        }
        let mut re = [0u8; 32];
        re.copy_from_slice(&message_1[..32]);
        self.0.handshake.remote_ephemeral_public_key = Some(PublicKey::Curve25519(re));
        self.0.mix_hash(&re)?;
        self.0.mix_hash(&message_1[32..])?;
        Ok(message_1[32..].to_vec())
    }

    /// Encode the second message to be sent
    pub fn encode_message_2<B: AsRef<[u8]>>(
        &mut self,
        payload: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        self.0.mix_hash(self.0.handshake.ephemeral_public_key)?;
        let hash = self.0.dh(
            self.0.handshake.ephemeral_secret_handle,
            *self
                .0
                .handshake
                .remote_ephemeral_public_key
                .as_ref()
                .unwrap(),
        )?;
        self.0.mix_key(&hash)?;

        let mut encrypted_s_and_tag = self
            .0
            .encrypt_and_mix_hash(self.0.handshake.static_public_key)?;
        let hash = self.0.dh(
            self.0.handshake.static_secret_handle,
            *self
                .0
                .handshake
                .remote_ephemeral_public_key
                .as_ref()
                .unwrap(),
        )?;
        self.0.mix_key(&hash)?;
        let mut encrypted_payload_and_tag = self.0.encrypt_and_mix_hash(payload)?;

        let mut output = self.0.handshake.ephemeral_public_key.as_ref().to_vec();
        output.append(&mut encrypted_s_and_tag);
        output.append(&mut encrypted_payload_and_tag);
        Ok(output)
    }

    /// Decode the final message received for the handshake
    pub fn decode_message_3<B: AsRef<[u8]>>(
        &mut self,
        message_3: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let message_3 = message_3.as_ref();
        let rs = self.0.decrypt_and_mix_hash(&message_3[..48])?;
        let rs = PublicKey::Curve25519(*array_ref![rs, 0, 32]);
        let hash = self.0.dh(self.0.handshake.ephemeral_secret_handle, rs)?;
        self.0.mix_key(hash)?;
        let payload = self.0.decrypt_and_mix_hash(&message_3[48..])?;
        self.0.handshake.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    /// Setup this responder to send and receive messages
    /// after decoding message 3
    pub fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        let keys = self.0.split()?;
        self.0
            .finalize(&keys[..AES256_KEYSIZE], &keys[AES256_KEYSIZE..])
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
pub struct XXInitiator<'a, V: Vault> {
    state: InitiatorState,
    initiator: Initiator<'a, V>,
}

impl<'a, V: Vault> XXInitiator<'a, V> {
    /// Create a new initiator using the provided backing vault
    pub fn new(vault: &'a mut V) -> Self {
        let ss = SymmetricState::new(vault);
        Self {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(ss),
        }
    }
}

impl<'a, V: Vault> KeyExchanger for XXInitiator<'a, V> {
    fn process<B: AsRef<[u8]>>(&mut self, data: B) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            InitiatorState::EncodeMessage1 => {
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

    fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
        match self.state {
            InitiatorState::Done => self.initiator.finalize(),
            _ => Err(VaultFailErrorKind::IOError.into()),
        }
    }
}

/// Represents an XX responder
#[derive(Debug)]
pub struct XXResponder<'a, V: Vault> {
    state: ResponderState,
    responder: Responder<'a, V>,
}

impl<'a, V: Vault> XXResponder<'a, V> {
    /// Create a new responder using the provided backing vault
    pub fn new(vault: &'a mut V) -> Self {
        let ss = SymmetricState::new(vault);
        Self {
            state: ResponderState::DecodeMessage1,
            responder: Responder(ss),
        }
    }
}

impl<'a, V: Vault> KeyExchanger for XXResponder<'a, V> {
    fn process<B: AsRef<[u8]>>(&mut self, data: B) -> Result<Vec<u8>, KexExchangeFailError> {
        match self.state {
            ResponderState::DecodeMessage1 => {
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

    fn finalize(&mut self) -> Result<CompletedKeyExchange, VaultFailError> {
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
        let mut vault = DefaultVault::default();
        let mut state = SymmetricState::new(&mut vault);
        let res = state.prologue();
        assert!(res.is_ok());
        assert_eq!(state.state.h, exp_h);
        assert_eq!(state.state.ck, *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0");
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

        let mut vault_init = DefaultVault::default();
        let mut vault_resp = DefaultVault::default();

        let ss_init = mock_prologue(&mut vault_init, INIT_STATIC, INIT_EPH);
        let ss_resp = mock_prologue(&mut vault_resp, RESP_STATIC, RESP_EPH);
        let mut initiator = XXInitiator {
            state: InitiatorState::EncodeMessage1,
            initiator: Initiator(ss_init),
        };
        let mut responder = XXResponder {
            state: ResponderState::DecodeMessage1,
            responder: Responder(ss_resp),
        };

        assert!(!initiator.is_complete());
        assert!(!responder.is_complete());
        let res = responder.process(&[]);
        assert!(res.is_err());
        let res = initiator.process(hex::decode(MSG_1_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg1 = res.unwrap();
        assert_eq!(hex::encode(&msg1), MSG_1_CIPHERTEXT);

        let res = responder.process(&msg1);
        assert!(res.is_ok());
        let res = responder.process(hex::decode(MSG_2_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg2 = res.unwrap();
        assert_eq!(hex::encode(&msg2), MSG_2_CIPHERTEXT);

        let res = initiator.process(msg2);
        assert!(res.is_ok());
        let res = initiator.process(hex::decode(MSG_3_PAYLOAD).unwrap());
        assert!(res.is_ok());
        let msg3 = res.unwrap();
        assert_eq!(hex::encode(&msg3), MSG_3_CIPHERTEXT);

        let res = responder.process(msg3);
        assert!(res.is_ok());

        let res = initiator.finalize();
        assert!(res.is_ok());
        let alice = res.unwrap();
        let res = responder.finalize();
        assert!(res.is_ok());
        let bob = res.unwrap();
        assert_eq!(alice.h, bob.h);
        let res =
            vault_init.aead_aes_gcm_encrypt(alice.encrypt_key, b"hello bob", &[0u8; 12], alice.h);
        assert!(res.is_ok());
        let ciphertext = res.unwrap();
        let res = vault_resp.aead_aes_gcm_decrypt(bob.decrypt_key, &ciphertext, &[0u8; 12], bob.h);
        assert!(res.is_ok());
        let plaintext = res.unwrap();
        assert_eq!(plaintext, b"hello bob");

        let res =
            vault_resp.aead_aes_gcm_encrypt(bob.encrypt_key, b"hello alice", &[1u8; 12], bob.h);
        assert!(res.is_ok());
        let ciphertext = res.unwrap();
        let res =
            vault_init.aead_aes_gcm_decrypt(alice.decrypt_key, &ciphertext, &[1u8; 12], alice.h);
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
        let mut vault_init = DefaultVault::default();
        let mut vault_resp = DefaultVault::default();

        let ss_init = mock_prologue(&mut vault_init, init_static, init_eph);
        let ss_resp = mock_prologue(&mut vault_resp, resp_static, resp_eph);
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

    fn mock_prologue<'a>(
        vault: &'a mut DefaultVault,
        static_private: &str,
        ephemeral_private: &str,
    ) -> SymmetricState<'a, DefaultVault> {
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        // Static x25519 for this handshake, `s`
        let bytes = hex::decode(static_private).unwrap();
        let key = SecretKey::Curve25519(*array_ref![bytes, 0, 32]);
        let static_secret_handle = vault.secret_import(&key, attributes).unwrap();
        let static_public_key = vault.secret_public_key_get(static_secret_handle).unwrap();

        // Ephemeral x25519 for this handshake, `e`
        let bytes = hex::decode(ephemeral_private).unwrap();
        let key = SecretKey::Curve25519(*array_ref![bytes, 0, 32]);
        let ephemeral_secret_handle = vault.secret_import(&key, attributes).unwrap();
        let ephemeral_public_key = vault
            .secret_public_key_get(ephemeral_secret_handle)
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
        SymmetricState {
            handshake: HandshakeStateData {
                static_public_key,
                static_secret_handle,
                ephemeral_public_key,
                ephemeral_secret_handle,
                remote_ephemeral_public_key: None,
                remote_static_public_key: None,
            },
            key: None,
            nonce,
            state: SymmetricStateData { h, ck },
            vault,
        }
    }
}
