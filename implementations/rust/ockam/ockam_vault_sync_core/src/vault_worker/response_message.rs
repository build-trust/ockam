use ockam_core::Message;
use ockam_vault_core::{
    Buffer, KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SmallBuffer,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ResultMessage<M> {
    inner: Result<M, u32>,
}

impl<M> ResultMessage<M>
where
    M: Message,
{
    pub fn inner(self) -> Result<M, u32> {
        self.inner
    }
}

impl<M> ResultMessage<M>
where
    M: Message,
{
    pub fn new(inner: Result<M, u32>) -> Self {
        ResultMessage { inner }
    }
}

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
