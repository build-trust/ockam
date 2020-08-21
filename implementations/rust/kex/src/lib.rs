#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unconditional_recursion,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_extern_crates,
    unused_parens,
    while_true
)]
//! Handles key exchange using Noise for Ockam channels

#[macro_use]
extern crate arrayref;

use ockam_vault::{
    error::{VaultFailError, VaultFailErrorKind},
    types::{
        PublicKey, SecretKey, SecretKeyAttributes, SecretKeyContext, SecretKeyType,
        SecretPersistenceType, SecretPurposeType,
    },
    Vault,
};

/// The maximum bytes that will be transmitted in a single message
pub const MAX_XX_TRANSMIT_SIZE: usize = 16384;
/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE: usize = 32;
/// The number of bytes in AES128 key
pub const AES128_KEYSIZE: usize = 16;
/// The number of bytes in AES256 key
pub const AES256_KEYSIZE: usize = 32;

/// Handles storing the current values for `h`, `ck`, and if its a key or not
#[derive(Copy, Clone, Debug)]
struct SymmetricStateData {
    h: [u8; SHA256_SIZE],
    ck: [u8; SHA256_SIZE],
}

impl Default for SymmetricStateData {
    fn default() -> Self {
        Self {
            h: [0u8; SHA256_SIZE],
            ck: [0u8; SHA256_SIZE],
        }
    }
}

/// A completed handshake transport
#[derive(Debug)]
pub struct TransportState<'a, V: Vault> {
    h: [u8; SHA256_SIZE],
    encrypt_key: SecretKeyContext,
    encrypt_nonce: u16,
    decrypt_key: SecretKeyContext,
    decrypt_nonce: u16,
    vault: &'a mut V,
}

/// The state of the handshake for a Noise session
#[derive(Copy, Clone, Debug)]
struct HandshakeStateData {
    ephemeral_public_key: PublicKey,
    ephemeral_secret_handle: SecretKeyContext,
    static_public_key: PublicKey,
    static_secret_handle: SecretKeyContext,
    remote_ephemeral_public_key: Option<PublicKey>,
    remote_static_public_key: Option<PublicKey>,
}

/// Represents the XX Handshake
#[derive(Debug)]
pub struct XXSymmetricState<'a, V: Vault> {
    handshake: HandshakeStateData,
    key: Option<SecretKeyContext>,
    nonce: u16,
    state: SymmetricStateData,
    vault: &'a mut V,
}

impl<'a, V: Vault> XXSymmetricState<'a, V> {
    const CSUITE: &'static [u8] = b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0";

    /// Create a new `HandshakeState` starting with the prologue
    pub fn prologue(vault: &'a mut V) -> Result<Self, VaultFailError> {
        let mut attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Persistent,
        };
        // 1. Generate a static 25519 keypair for this handshake and set it to `s`
        let static_secret_handle = vault.secret_generate(attributes)?;
        let static_public_key = vault.secret_public_key_get(static_secret_handle)?;

        attributes.persistence = SecretPersistenceType::Ephemeral;
        // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
        let ephemeral_secret_handle = vault.secret_generate(attributes)?;
        let ephemeral_public_key = vault.secret_public_key_get(ephemeral_secret_handle)?;

        // 3. Set k to empty, Set n to 0
        let nonce = 0;

        // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let mut h = [0u8; SHA256_SIZE];
        h[..Self::CSUITE.len()].copy_from_slice(Self::CSUITE);
        let ck = h;
        let h = vault.sha256(h)?;
        Ok(Self {
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
        })
    }

    /// Perform the diffie-hellman computation
    pub fn dh(
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
    pub fn mix_key<B: AsRef<[u8]>>(&mut self, hash: B) -> Result<(), VaultFailError> {
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
    pub fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError> {
        let mut input = self.state.h.to_vec();
        input.extend_from_slice(data.as_ref());
        self.state.h = self.vault.sha256(&input)?;
        Ok(())
    }

    /// Encrypt and mix step in Noise protocol
    pub fn encrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        plaintext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext_and_tag = self.vault.aead_aes_gcm_encrypt(
            self.key
                .ok_or_else(|| VaultFailErrorKind::AeadAesGcmEncrypt)?,
            plaintext,
            nonce.as_ref(),
            &self.state.h,
        )?;
        self.mix_hash(&ciphertext_and_tag)?;
        self.nonce += 1;
        Ok(ciphertext_and_tag)
    }

    /// Decrypt and mix step in Noise protocol
    pub fn decrypt_and_mix_hash<B: AsRef<[u8]>>(
        &mut self,
        ciphertext: B,
    ) -> Result<Vec<u8>, VaultFailError> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.nonce.to_be_bytes());
        let ciphertext = ciphertext.as_ref();
        let plaintext = self.vault.aead_aes_gcm_decrypt(
            self.key
                .ok_or_else(|| VaultFailErrorKind::AeadAesGcmDecrypt)?,
            ciphertext,
            nonce.as_ref(),
            &self.state.h,
        )?;
        self.mix_hash(ciphertext)?;
        self.nonce += 1;
        Ok(plaintext)
    }

    /// Split step in Noise protocol
    pub fn split(&mut self) -> Result<Vec<u8>, VaultFailError> {
        self.vault
            .hkdf_sha256(self.state.ck.as_ref(), &[], AES256_KEYSIZE + AES256_KEYSIZE)
    }

    /// Set this state up to send and receive messages
    pub fn finalize<'b, VV: Vault>(
        &mut self,
        vault: &'b mut VV,
        encrypt_ref: &[u8],
        decrypt_ref: &[u8],
    ) -> Result<TransportState<'b, VV>, VaultFailError> {
        debug_assert_eq!(encrypt_ref.len(), AES256_KEYSIZE);
        debug_assert_eq!(decrypt_ref.len(), AES256_KEYSIZE);
        let keys = self.split()?;
        let mut decrypt = [0u8; AES256_KEYSIZE];
        let mut encrypt = [0u8; AES256_KEYSIZE];
        decrypt.copy_from_slice(&keys[..AES256_KEYSIZE]);
        encrypt.copy_from_slice(&keys[AES256_KEYSIZE..(2 * AES256_KEYSIZE)]);
        let decrypt = SecretKey::Aes256(decrypt);
        let encrypt = SecretKey::Aes256(encrypt);
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Aes256,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral,
        };
        let decrypt_key = vault.secret_import(&decrypt, attributes)?;
        let encrypt_key = vault.secret_import(&encrypt, attributes)?;
        Ok(TransportState {
            h: self.state.h,
            encrypt_key,
            encrypt_nonce: 0,
            decrypt_key,
            decrypt_nonce: 0,
            vault,
        })
    }
}

/// Provides methods for handling the initiator role
#[derive(Debug)]
pub struct Initiator<'a, V: Vault>(XXSymmetricState<'a, V>);

impl<'a, V: Vault> Initiator<'a, V> {
    /// Wrap a symmetric state to run as the Initiator
    pub fn new(ss: XXSymmetricState<'a, V>) -> Self {
        Self(ss)
    }

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
    pub fn finalize<'b, VV: Vault>(
        &mut self,
        vault: &'b mut VV,
    ) -> Result<TransportState<'b, VV>, VaultFailError> {
        let keys = self.0.split()?;
        self.0
            .finalize(vault, &keys[AES256_KEYSIZE..], &keys[..AES256_KEYSIZE])
    }
}

/// Provides methods for handling the responder role
#[derive(Debug)]
pub struct Responder<'a, V: Vault>(XXSymmetricState<'a, V>);

impl<'a, V: Vault> Responder<'a, V> {
    /// Wrap a symmetric state to run as the Responder
    pub fn new(ss: XXSymmetricState<'a, V>) -> Self {
        Self(ss)
    }

    /// Decode the first message sent
    pub fn decode_message_1<B: AsRef<[u8]>>(&mut self, message_1: B) -> Result<(), VaultFailError> {
        let message_1 = message_1.as_ref();
        if message_1.len() < 32 {
            return Err(VaultFailErrorKind::SecretSizeMismatch.into());
        }
        let mut re = [0u8; 32];
        re.copy_from_slice(&message_1[..32]);
        self.0.handshake.remote_ephemeral_public_key = Some(PublicKey::Curve25519(re));
        self.0.mix_hash(&re)?;
        self.0.mix_hash(&message_1[32..])?;
        Ok(())
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
    pub fn finalize<'b, VV: Vault>(
        &mut self,
        vault: &'b mut VV,
    ) -> Result<TransportState<'b, VV>, VaultFailError> {
        let keys = self.0.split()?;
        self.0
            .finalize(vault, &keys[..AES256_KEYSIZE], &keys[AES256_KEYSIZE..])
    }
}

/// Errors thrown by Key exchange
pub mod error;

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
        let res = XXSymmetricState::prologue(&mut vault);
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.state.h, exp_h);
        assert_eq!(ss.state.ck, *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0");
        assert_eq!(ss.nonce, 0);
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
        let mut initiator = Initiator::new(ss_init);
        let mut responder = Responder::new(ss_resp);

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

        let mut vault_initiator = DefaultVault::default();
        let mut vault_responder = DefaultVault::default();
        let res = initiator.finalize(&mut vault_initiator);
        assert!(res.is_ok());
        let res = responder.finalize(&mut vault_responder);
        assert!(res.is_ok());
    }

    fn mock_prologue<'a>(
        vault: &'a mut DefaultVault,
        static_private: &str,
        ephemeral_private: &str,
    ) -> XXSymmetricState<'a, DefaultVault> {
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
        XXSymmetricState {
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
