#![deny(
missing_docs,
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

/// The state of the handshake for a Noise session
#[derive(Copy, Clone, Debug)]
struct HandshakeStateData {
    ephemeral_public_key: PublicKey,
    ephemeral_secret_handle: SecretKeyContext,
    static_public_key: PublicKey,
    static_secret_handle: SecretKeyContext,
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
                ephemeral_secret_handle
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
    pub fn encrypt_and_mix_hash<B: AsRef<[u8]>, C: AsMut<[u8]>>(&mut self, plaintext: B) -> Result<Vec<u8>, VaultFailError> {
        self.vault.aead_aes_gcm_encrypt(self.key.ok_or_else(|| VaultFailErrorKind::AeadAesGcmEncrypt)?, plaintext, self.nonce.to_be_bytes().as_ref(), &self.state.h)
    }

    /// Decrypt and mix step in Noise protocol
    pub fn decrypt_and_mix_hash<B: AsRef<[u8]>, C: AsMut<[u8]>>(&mut self, ciphertext: B) -> Result<Vec<u8>, VaultFailError> {
        self.vault.aead_aes_gcm_decrypt(self.key.ok_or_else(|| VaultFailErrorKind::AeadAesGcmDecrypt)?, ciphertext, self.nonce.to_be_bytes().as_ref(), &self.state.h)
    }

    /// Split step in Noise protocol
    pub fn split(&mut self) -> Result<Vec<u8>, VaultFailError> {
        self.vault.hkdf_sha256(self.state.ck.as_ref(), &[], SHA256_SIZE + AES256_KEYSIZE)
    }
}

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
