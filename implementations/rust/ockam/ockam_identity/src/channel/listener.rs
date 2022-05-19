use crate::{DecryptorWorker, IdentityTrait, TrustPolicy};
use ockam_channel::CreateResponderChannelMessage;
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{Result, Routed, Worker};
use ockam_key_exchange_xx::XXVault;
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<I: IdentityTrait, V: XXVault> {
    trust_policy: Arc<dyn TrustPolicy>,
    identity: I,
    vault: V,
}

impl<I: IdentityTrait, V: XXVault> IdentityChannelListener<I, V> {
    pub fn new(trust_policy: impl TrustPolicy, identity: I, vault: V) -> Self {
        IdentityChannelListener {
            trust_policy: Arc::new(trust_policy),
            identity,
            vault,
        }
    }
}

#[ockam_core::worker]
impl<I: IdentityTrait, V: XXVault> Worker for IdentityChannelListener<I, V> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let trust_policy = Arc::clone(&self.trust_policy);
        let identity = self.identity.async_try_clone().await?;
        let vault = self.vault.async_try_clone().await?;
        DecryptorWorker::create_responder(ctx, identity, trust_policy, vault, msg).await
    }
}
