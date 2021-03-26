use crate::{AsymmetricVault, Hasher, KeyIdVault, SecretVault, Signer, SymmetricVault, Verifier};
use async_trait::async_trait;
use ockam_core::{Result, Routed, Worker, Address};
use ockam_node::Context;
use zeroize::Zeroize;

pub trait VaultTrait:
    AsymmetricVault
    + Hasher
    + KeyIdVault
    + SecretVault
    + Signer
    + SymmetricVault
    + Verifier
    + Send
    + 'static
{
}

impl<V> VaultTrait for V where
    V: AsymmetricVault
        + Hasher
        + KeyIdVault
        + SecretVault
        + Signer
        + SymmetricVault
        + Verifier
        + Send
        + 'static
{
}

mod request_message;
pub use request_message::*;

mod response_message;
pub use response_message::*;
use rand::random;

#[derive(Zeroize)]
pub struct VaultWorker<V>
where
    V: VaultTrait,
{
    inner: V,
}

impl<V> VaultWorker<V>
where
    V: VaultTrait,
{
    pub fn new(inner: V) -> Self {
        Self { inner }
    }

    pub async fn start(ctx: &Context, inner: V) -> Result<Address> {
        let address: Address = random();

        ctx.start_worker(address.clone(), Self::new(inner)).await?;

        Ok(address)
    }
}

#[async_trait]
impl<V> Worker for VaultWorker<V>
where
    V: VaultTrait,
{
    type Message = VaultRequestMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let reply = msg.reply();
        // TODO: Return errors
        // TODO: Return specific enum options instead of enum itself
        // TODO: Add request_id to requests and responses
        let response = match msg.take() {
            VaultRequestMessage::EcDiffieHellman {
                context,
                peer_public_key,
            } => {
                let res = self
                    .inner
                    .ec_diffie_hellman(&context, peer_public_key.as_ref())?;
                VaultResponseMessage::EcDiffieHellman(res)
            }
            VaultRequestMessage::Sha256 { data } => {
                let res = self.inner.sha256(&data)?;
                VaultResponseMessage::Sha256(res)
            }
            VaultRequestMessage::HkdfSha256 {
                salt,
                info,
                ikm,
                output_attributes,
            } => {
                let res = self
                    .inner
                    .hkdf_sha256(&salt, &info, ikm.as_ref(), output_attributes)?;
                VaultResponseMessage::HkdfSha256(res)
            }
            VaultRequestMessage::GetSecretByKeyId { key_id } => {
                let res = self.inner.get_secret_by_key_id(&key_id)?;
                VaultResponseMessage::GetSecretByKeyId(res)
            }
            VaultRequestMessage::ComputeKeyIdForPublicKey { public_key } => {
                let res = self.inner.compute_key_id_for_public_key(&public_key)?;
                VaultResponseMessage::ComputeKeyIdForPublicKey(res)
            }
            VaultRequestMessage::SecretGenerate { attributes } => {
                let res = self.inner.secret_generate(attributes)?;
                VaultResponseMessage::SecretGenerate(res)
            }
            VaultRequestMessage::SecretImport { secret, attributes } => {
                let res = self.inner.secret_import(&secret, attributes)?;
                VaultResponseMessage::SecretImport(res)
            }
            VaultRequestMessage::SecretExport { context } => {
                let res = self.inner.secret_export(&context)?;
                VaultResponseMessage::SecretExport(res)
            }
            VaultRequestMessage::SecretAttributesGet { context } => {
                let res = self.inner.secret_attributes_get(&context)?;
                VaultResponseMessage::SecretAttributesGet(res)
            }
            VaultRequestMessage::SecretPublicKeyGet { context } => {
                let res = self.inner.secret_public_key_get(&context)?;
                VaultResponseMessage::SecretPublicKeyGet(res)
            }
            VaultRequestMessage::SecretDestroy { context } => {
                self.inner.secret_destroy(context)?;
                VaultResponseMessage::SecretDestroy
            }
            VaultRequestMessage::Sign { secret_key, data } => {
                let res = self.inner.sign(&secret_key, &data)?;
                VaultResponseMessage::Sign(res)
            }
            VaultRequestMessage::AeadAesGcmEncrypt {
                context,
                plaintext,
                nonce,
                aad,
            } => {
                let res = self
                    .inner
                    .aead_aes_gcm_encrypt(&context, &plaintext, &nonce, &aad)?;
                VaultResponseMessage::AeadAesGcmEncrypt(res)
            }
            VaultRequestMessage::AeadAesGcmDecrypt {
                context,
                cipher_text,
                nonce,
                aad,
            } => {
                let res = self
                    .inner
                    .aead_aes_gcm_decrypt(&context, &cipher_text, &nonce, &aad)?;
                VaultResponseMessage::AeadAesGcmDecrypt(res)
            }
            VaultRequestMessage::Verify {
                signature,
                public_key,
                data,
            } => {
                let res = self
                    .inner
                    .verify(&signature, public_key.as_ref(), &data)
                    .is_ok();
                VaultResponseMessage::Verify(res)
            }
        };

        ctx.send_message(reply, response).await?;

        Ok(())
    }
}
