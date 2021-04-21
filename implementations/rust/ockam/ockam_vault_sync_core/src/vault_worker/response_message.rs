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
    pub fn inner(self, error_domain: &'static str) -> ockam_core::Result<M> {
        self.inner
            .map_err(|e| ockam_core::Error::new(e, error_domain))
    }
}

impl<M> ResultMessage<M>
where
    M: Message,
{
    pub fn new(inner: ockam_core::Result<M>) -> Self {
        Self {
            inner: inner.map_err(|e| e.code()),
        }
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
