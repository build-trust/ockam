use ockam_core::vault::{
    Buffer, KeyId, PublicKey, Secret, SecretAttributes, SecretKey, Signature, SmallBuffer,
};
use ockam_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Message)]
pub enum VaultResponseMessage {
    EcDiffieHellman(Secret),
    Sha256([u8; 32]),
    HkdfSha256(SmallBuffer<Secret>),
    GetSecretByKeyId(Secret),
    ComputeKeyIdForPublicKey(KeyId),
    SecretGenerate(Secret),
    SecretImport(Secret),
    SecretExport(SecretKey),
    SecretAttributesGet(SecretAttributes),
    SecretPublicKeyGet(PublicKey),
    SecretDestroy,
    Sign(Signature),
    AeadAesGcmEncrypt(Buffer<u8>),
    AeadAesGcmDecrypt(Buffer<u8>),
    Verify(bool),
}
