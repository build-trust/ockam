use crate::{ChannelKeys, SecureChannelError, SecureChannelVault};
use ockam_core::async_trait;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{Any, Encodable, Result, Route, Routed, TransportMessage, Worker};
use ockam_node::Context;
use tracing::debug;

pub(crate) struct SecureChannelEncryptor<V: SecureChannelVault> {
    keys: ChannelKeys,
    remote_route: Route,
    vault: V,
}

impl<V: SecureChannelVault> SecureChannelEncryptor<V> {
    pub(crate) fn new(keys: ChannelKeys, remote_route: Route, vault: V) -> Self {
        Self {
            keys,
            remote_route,
            vault,
        }
    }

    /// We use u64 nonce since it's convenient to work with it (e.g. increment)
    /// But we use 8-byte be format to send it over to the other side (according to noise spec)
    /// And we use 12-byte be format for encryption, since AES-GCM wants 12 bytes
    pub(crate) fn convert_nonce_from_u64(nonce: u64) -> ([u8; 8], [u8; 12]) {
        let mut n: [u8; 12] = [0; 12];
        let b: [u8; 8] = nonce.to_be_bytes();

        n[4..].copy_from_slice(&b);

        (b, n)
    }

    async fn handle_encrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!("SecureChannel received Encrypt");

        let reply = msg.return_route();
        let mut onward_route = msg.onward_route();
        let transport_message = msg.into_transport_message();
        let payload = transport_message.payload;

        let _ = onward_route.step();

        let msg = TransportMessage::v1(onward_route, reply, payload.to_vec());
        let payload = msg.encode()?;

        let payload = {
            let nonce = self.keys.nonce;

            if nonce == u64::MAX {
                return Err(SecureChannelError::InvalidNonce.into());
            }

            self.keys.nonce += 1;

            let (small_nonce, nonce) = Self::convert_nonce_from_u64(nonce);

            let mut cipher_text = self
                .vault
                .aead_aes_gcm_encrypt(&self.keys.key, payload.as_slice(), &nonce, &[])
                .await?;

            let mut res = Vec::new();
            res.extend_from_slice(&small_nonce);
            res.append(&mut cipher_text);

            res
        };

        ctx.send(self.remote_route.clone(), payload).await
    }
}

#[async_trait]
impl<V: SecureChannelVault> Worker for SecureChannelEncryptor<V> {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.handle_encrypt(ctx, msg).await
    }
}
