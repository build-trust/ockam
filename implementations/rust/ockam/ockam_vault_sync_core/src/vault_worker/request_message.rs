use ockam_core::vault::{Buffer, PublicKey, Secret, SecretAttributes, Signature, SmallBuffer};
use ockam_core::{compat::string::String, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Message)]
pub enum VaultRequestMessage {
    EcDiffieHellman {
        context: Secret,
        peer_public_key: PublicKey,
    },
    Sha256 {
        data: Buffer<u8>,
    },
    HkdfSha256 {
        salt: Secret,
        info: Buffer<u8>,
        ikm: Option<Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    },
    GetSecretByKeyId {
        key_id: String,
    },
    ComputeKeyIdForPublicKey {
        public_key: PublicKey,
    },
    SecretGenerate {
        attributes: SecretAttributes,
    },
    SecretImport {
        secret: Buffer<u8>,
        attributes: SecretAttributes,
    },
    SecretExport {
        context: Secret,
    },
    SecretAttributesGet {
        context: Secret,
    },
    SecretPublicKeyGet {
        context: Secret,
    },
    SecretDestroy {
        context: Secret,
    },
    Sign {
        secret_key: Secret,
        data: Buffer<u8>,
    },
    AeadAesGcmEncrypt {
        context: Secret,
        plaintext: Buffer<u8>,
        nonce: Buffer<u8>,
        aad: Buffer<u8>,
    },
    AeadAesGcmDecrypt {
        context: Secret,
        cipher_text: Buffer<u8>,
        nonce: Buffer<u8>,
        aad: Buffer<u8>,
    },
    Verify {
        signature: Signature,
        public_key: PublicKey,
        data: Buffer<u8>,
    },
}
