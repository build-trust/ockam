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
//!

#[macro_use]
extern crate arrayref;

use ockam_vault::{Vault, error::{
    VaultFailError,
    VaultFailErrorKind,
}, types::{
    SecretKeyContext, SecretKeyAttributes, SecretKeyType, SecretPersistenceType, SecretPurposeType, SecretKey, PublicKey
}};

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
    vault: &'a mut V
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
    vault: &'a mut V
}

impl<'a, V: Vault> XXSymmetricState<'a, V> {
    const CSUITE: &'static [u8] = b"Noise_XX_25519_AESGCM_SHA256";

    /// Create a new `HandshakeState` starting with the prologue
    pub fn prologue(vault: &'a mut V) -> Result<Self, VaultFailError> {
        let attributes = SecretKeyAttributes {
            xtype: SecretKeyType::Curve25519,
            purpose: SecretPurposeType::KeyAgreement,
            persistence: SecretPersistenceType::Ephemeral
        };
        // 1. Generate a static 25519 keypair for this handshake and set it to `s`
        let static_secret_handle = vault.secret_generate(attributes)?;
        let static_public_key = vault.secret_public_key_get(static_secret_handle)?;

        // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
        let ephemeral_secret_handle = vault.secret_generate(attributes)?;
        let ephemeral_public_key = vault.secret_public_key_get(ephemeral_secret_handle)?;

        // 3. Set k to empty, Set n to 0
        let nonce = 0;

        // 4. Set h and ck to 'Noise_XX_25519_AESGCM_SHA256'
        // 5. h = SHA256(h || prologue),
        // prologue is empty
        // mix_hash(xx, NULL, 0);
        let h = vault.sha256(Self::CSUITE)?;
        let ck = h.clone();
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
            state: SymmetricStateData {
                h,
                ck,
            },
            vault
        })
    }

    /// mix key step in Noise protocol
    pub fn mix_key<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError> {
        let hash = self.vault.hkdf_sha256(&self.state.ck[..], data, SHA256_SIZE + AES256_KEYSIZE)?;
        self.state.ck = *array_ref![hash, 0, SHA256_SIZE];
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
        Ok(())
    }

    /// mix hash step in Noise protocol
    pub fn mix_hash<B: AsRef<[u8]>>(&mut self, data: B) -> Result<(), VaultFailError> {
        let mut input = Self::CSUITE.to_vec();
        input.extend_from_slice(data.as_ref());
        self.state.h = self.vault.sha256(&input)?;
        Ok(())
    }

    /// Encrypt and mix step in Noise protocol
    pub fn encrypt_and_mix_hash<B: AsRef<[u8]>>(&mut self, plaintext: B) -> Result<Vec<u8>, VaultFailError> {
        self.vault.aead_aes_gcm_encrypt(self.key.ok_or_else(|| VaultFailErrorKind::AeadAesGcmEncrypt)?, plaintext, self.nonce.to_be_bytes().as_ref(), &self.state.h)
    }

    /// Decrypt and mix step in Noise protocol
    pub fn decrypt_and_mix_hash<B: AsRef<[u8]>>(&mut self, ciphertext: B) -> Result<Vec<u8>, VaultFailError> {
        self.vault.aead_aes_gcm_decrypt(self.key.ok_or_else(|| VaultFailErrorKind::AeadAesGcmDecrypt)?, ciphertext, self.nonce.to_be_bytes().as_ref(), &self.state.h)
    }

    /// Split step in Noise protocol
    pub fn split(&mut self) -> Result<Vec<u8>, VaultFailError> {
        self.vault.hkdf_sha256(self.state.ck.as_ref(), &[], AES256_KEYSIZE + AES256_KEYSIZE)
    }

    /// Set this state up to send and receive messages
    pub fn finalize<'b, VV: Vault>(&mut self, vault: &'b mut VV) -> Result<TransportState<'b, VV>, VaultFailError> {
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
            vault
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
    pub fn encode_message_1<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>, VaultFailError> {
        let payload = payload.as_ref();
        self.0.mix_hash(self.0.handshake.ephemeral_public_key)?;
        self.0.mix_hash(payload)?;

        let mut output = self.0.handshake.ephemeral_public_key.as_ref().to_vec();
        output.extend_from_slice(payload);
        Ok(output)
    }

    /// Decode the second message in the sequence, sent from the responder
    pub fn decode_message_2<B: AsRef<[u8]>>(&mut self, message: B) -> Result<Vec<u8>, VaultFailError> {
        let message = message.as_ref();
        let mut re = [0u8; 32];
        re.copy_from_slice(&message[..32]);
        let encrypted_rs_and_tag = &message[32..80];
        let encrypted_payload_and_tag = &message[80..];

        let re = PublicKey::Curve25519(re);
        self.0.handshake.remote_ephemeral_public_key = Some(re);

        self.0.mix_hash(&re)?;
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.ephemeral_secret_handle, re)?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let rs = self.0.decrypt_and_mix_hash(encrypted_rs_and_tag)?;
        let rs = PublicKey::Curve25519(*array_ref![rs, 0, 32]);
        self.0.handshake.remote_static_public_key = Some(rs);
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.ephemeral_secret_handle, rs)?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let payload = self.0.decrypt_and_mix_hash(encrypted_payload_and_tag)?;
        Ok(payload)
    }

    /// Encode the final message to be sent
    pub fn encode_message_3<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>, VaultFailError> {
        let mut encrypted_s_and_tag = self.0.encrypt_and_mix_hash(self.0.handshake.static_public_key)?;
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.static_secret_handle, *self.0.handshake.remote_ephemeral_public_key.as_ref().unwrap())?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let mut encrypted_payload_and_tag = self.0.encrypt_and_mix_hash(payload)?;
        encrypted_s_and_tag.append(&mut encrypted_payload_and_tag);
        Ok(encrypted_s_and_tag)
    }

    /// Setup this initiator to send and receive messages
    /// after encoding message 3
    pub fn finalize<'b, VV: Vault>(&mut self, vault: &'b mut VV) -> Result<TransportState<'b, VV>, VaultFailError> {
        self.0.finalize(vault)
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
    pub fn encode_message_2<B: AsRef<[u8]>>(&mut self, payload: B) -> Result<Vec<u8>, VaultFailError> {
        self.0.mix_hash(self.0.handshake.ephemeral_public_key)?;
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.ephemeral_secret_handle, *self.0.handshake.remote_ephemeral_public_key.as_ref().unwrap())?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let mut encrypted_s_and_tag = self.0.encrypt_and_mix_hash(self.0.handshake.static_public_key)?;
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.static_secret_handle, *self.0.handshake.remote_ephemeral_public_key.as_ref().unwrap())?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let mut encrypted_payload_and_tag = self.0.encrypt_and_mix_hash(payload)?;

        let mut output = self.0.handshake.ephemeral_public_key.as_ref().to_vec();
        output.append(&mut encrypted_s_and_tag);
        output.append(&mut encrypted_payload_and_tag);
        Ok(output)
    }

    /// Decode the final message received for the handshake
    pub fn decode_message_3<B: AsRef<[u8]>>(&mut self, message_3: B) -> Result<Vec<u8>, VaultFailError> {
        let message_3 = message_3.as_ref();
        let rs = self.0.decrypt_and_mix_hash(&message_3[..48])?;
        let rs = PublicKey::Curve25519(*array_ref![rs, 0, 32]);
        let shared_secret_ctx = self.0.vault.ec_diffie_hellman(self.0.handshake.ephemeral_secret_handle, rs)?;
        let shared_secret = self.0.vault.secret_export(shared_secret_ctx)?;
        self.0.mix_key(shared_secret)?;
        let payload = self.0.decrypt_and_mix_hash(&message_3[48..])?;
        self.0.handshake.remote_static_public_key = Some(rs);
        Ok(payload)
    }

    /// Setup this responder to send and receive messages
    /// after decoding message 3
    pub fn finalize<'b, VV: Vault>(&mut self, vault: &'b mut VV) -> Result<TransportState<'b, VV>, VaultFailError> {
        self.0.finalize(vault)
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
        let exp_h = [ 106, 46, 40, 203, 12, 213, 250, 236, 181, 181, 143, 65, 101, 32, 71, 73, 245, 126, 152, 127, 140, 234, 95, 77, 44, 142, 231, 83, 57, 81, 35, 37, ];
        let mut vault = DefaultVault::default();
        let res = XXSymmetricState::prologue(&mut vault);
        assert!(res.is_ok());
        let ss = res.unwrap();
        assert_eq!(ss.state.h, exp_h);
        assert_eq!(ss.state.ck, exp_h);
        assert_eq!(ss.nonce, 0);
    }
}
