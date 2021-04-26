use arrayref::array_ref;
use ockam_core::hex::encode;
use ockam_core::lib::convert::TryFrom;
use ockam_vault_core::{
    AsymmetricVault, Hasher, PublicKey, SecretVault, Signer, SymmetricVault, Verifier,
};

mod error;
pub use error::*;

mod initiator;
pub use initiator::*;
mod responder;
pub use responder::*;
mod new_key_exchanger;
pub use new_key_exchanger::*;

/// Represents and (X)EdDSA or ECDSA signature
/// from Ed25519 or P-256
#[derive(Clone, Copy)]
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

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Signature {{ {} }}", encode(self.0.as_ref()))
    }
}

/// Represents all the keys and signature to send to an enrollee
#[derive(Clone, Debug)]
pub struct PreKeyBundle {
    identity_key: PublicKey,
    signed_prekey: PublicKey,
    signature_prekey: Signature,
    one_time_prekey: PublicKey,
}

impl PreKeyBundle {
    const SIZE: usize = 32 + 32 + 64 + 32;
    /// Convert the prekey bundle to a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(self.identity_key.as_ref());
        output.extend_from_slice(self.signed_prekey.as_ref());
        output.extend_from_slice(self.signature_prekey.0.as_ref());
        output.extend_from_slice(self.one_time_prekey.as_ref());
        output
    }
}

impl TryFrom<&[u8]> for PreKeyBundle {
    type Error = ockam_core::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::SIZE {
            return Err(X3DHError::MessageLenMismatch.into());
        }
        let identity_key = PublicKey::new(array_ref![data, 0, 32].to_vec());
        let signed_prekey = PublicKey::new(array_ref![data, 32, 32].to_vec());
        let signature_prekey = Signature(*array_ref![data, 64, 64]);
        let one_time_prekey = PublicKey::new(array_ref![data, 128, 32].to_vec());
        Ok(Self {
            identity_key,
            signed_prekey,
            signature_prekey,
            one_time_prekey,
        })
    }
}

const CSUITE: &[u8] = b"X3DH_25519_AESGCM_SHA256\0\0\0\0\0\0\0\0";
/// EK, Hash(EIK), IK, EdDSA, AES_GCM_TAG
const ENROLLMENT_MSG_SIZE: usize = 32 + 32 + 32 + 64 + 16;

/// Vault with X3DH required functionality
pub trait X3dhVault:
    SecretVault + Signer + Verifier + AsymmetricVault + SymmetricVault + Hasher + Clone + Send + 'static
{
}

impl<D> X3dhVault for D where
    D: SecretVault
        + Signer
        + Verifier
        + AsymmetricVault
        + SymmetricVault
        + Hasher
        + Clone
        + Send
        + 'static
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_key_exchange_core::KeyExchanger;
    use ockam_vault::SoftwareVault;
    use ockam_vault_sync_core::VaultMutex;

    #[test]
    fn handshake() {
        let mut vault = VaultMutex::create(SoftwareVault::default());
        let mut initiator = Initiator::new(vault.clone(), None);
        let mut responder = Responder::new(vault.clone(), None);

        let res = initiator.process(&[]);
        assert!(res.is_ok());
        let eik_bytes = res.unwrap();
        assert_eq!(eik_bytes.len(), 32);
        let res = responder.process(&[]);
        assert!(res.is_ok());
        let prekey_bundle_bytes = res.unwrap();

        let res = initiator.process(prekey_bundle_bytes.as_slice());
        assert!(res.is_ok(), "{:?}", res);
        let final_message = res.unwrap();

        let res = responder.process(eik_bytes.as_slice());
        assert!(res.is_ok(), "{:?}", res);
        let res = responder.process(final_message.as_slice());
        assert!(res.is_ok(), "{:?}", res);

        let init = initiator.finalize().unwrap();
        let resp = responder.finalize().unwrap();

        let ciphertext_and_tag = vault
            .aead_aes_gcm_encrypt(init.encrypt_key(), b"Hello Alice", &[1u8; 12], &[])
            .unwrap();
        let plaintext = vault
            .aead_aes_gcm_decrypt(
                resp.decrypt_key(),
                ciphertext_and_tag.as_slice(),
                &[1u8; 12],
                &[],
            )
            .unwrap();
        assert_eq!(plaintext, b"Hello Alice");
    }
}
