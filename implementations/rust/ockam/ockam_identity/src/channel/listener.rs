use crate::{IdentityTrait, SecureChannelWorker, TrustPolicy};
use ockam_channel::{CreateResponderChannelMessage, SecureChannel};
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<I: IdentityTrait, V: XXVault> {
    trust_policy: Arc<dyn TrustPolicy>,
    identity: I,
    vault: V,
    listener_address: Address,
}

impl<I: IdentityTrait, V: XXVault> IdentityChannelListener<I, V> {
    pub fn new(trust_policy: impl TrustPolicy, identity: I, vault: V) -> Self {
        let listener_address = Address::random_local();
        IdentityChannelListener {
            trust_policy: Arc::new(trust_policy),
            identity,
            vault,
            listener_address,
        }
    }
}

#[ockam_core::worker]
impl<I: IdentityTrait, V: XXVault> Worker for IdentityChannelListener<I, V> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.async_try_clone().await?);
        let vault = self.vault.async_try_clone().await?;
        SecureChannel::create_listener_extended(
            ctx,
            self.listener_address.clone(),
            new_key_exchanger,
            vault,
        )
        .await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // Ignore the error in case node is shutting down and this listener was stopped already
        let _ = ctx.stop_worker(self.listener_address.clone()).await;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let trust_policy = Arc::clone(&self.trust_policy);
        let identity = self.identity.async_try_clone().await?;
        SecureChannelWorker::create_responder(
            ctx,
            identity,
            trust_policy,
            self.listener_address.clone(),
            msg,
        )
        .await
    }
}
