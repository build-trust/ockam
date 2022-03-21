use crate::{IdentityTrait, SecureChannelWorker, TrustPolicy};
use ockam_channel::{CreateResponderChannelMessage, SecureChannel};
use ockam_core::compat::boxed::Box;
use ockam_core::compat::rand::random;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener<T: TrustPolicy, I: IdentityTrait, V: XXVault> {
    trust_policy: T,
    identity: I,
    vault: V,
    listener_address: Address,
}

impl<T: TrustPolicy, I: IdentityTrait, V: XXVault> IdentityChannelListener<T, I, V> {
    pub fn new(trust_policy: T, identity: I, vault: V) -> Self {
        let listener_address: Address = random();
        IdentityChannelListener {
            trust_policy,
            identity,
            vault,
            listener_address,
        }
    }
}

#[ockam_core::worker]
impl<T: TrustPolicy, I: IdentityTrait, V: XXVault> Worker for IdentityChannelListener<T, I, V> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.async_try_clone().await?);
        SecureChannel::create_listener_extended(
            ctx,
            self.listener_address.clone(),
            new_key_exchanger,
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
        let trust_policy = self.trust_policy.async_try_clone().await?;
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
