use crate::{Buffer, KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SmallBuffer};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
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
    Sign(#[serde(with = "BigArray")] [u8; 64]),
    AeadAesGcmEncrypt(Buffer<u8>),
    AeadAesGcmDecrypt(Buffer<u8>),
    Verify(bool),
}
