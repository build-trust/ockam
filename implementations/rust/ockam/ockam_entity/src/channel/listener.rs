use crate::{Identity, SecureChannelWorker, TrustPolicy};
use ockam_channel::{CreateResponderChannelMessage, SecureChannel};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::rand::random;
use ockam_core::traits::AsyncClone;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;

pub(crate) struct ProfileChannelListener<
    T: TrustPolicy,
    P: Identity + AsyncClone + Clone,
    V: XXVault + Sync,
> {
    trust_policy: T,
    profile: P,
    vault: V,
    listener_address: Address,
}

#[async_trait]
impl<T: TrustPolicy + Sync, P: Identity + AsyncClone + Clone + Sync, V: XXVault + Sync> AsyncClone
    for ProfileChannelListener<T, P, V>
{
    async fn async_clone(&self) -> Self {
        Self {
            trust_policy: self.trust_policy.async_clone().await,
            profile: self.profile.async_clone().await,
            vault: self.vault.async_clone().await,
            listener_address: self.listener_address.clone(),
        }
    }
}

impl<T: TrustPolicy, P: Identity + AsyncClone + Clone, V: XXVault + Sync>
    ProfileChannelListener<T, P, V>
{
    pub fn new(trust_policy: T, profile: P, vault: V) -> Self {
        let listener_address: Address = random();
        ProfileChannelListener {
            trust_policy,
            profile,
            vault,
            listener_address,
        }
    }
}

#[ockam_core::worker]
impl<
        T: TrustPolicy + Sync,
        P: Identity + AsyncClone + Clone + Sync,
        V: XXVault + AsyncClone + Sync,
    > Worker for ProfileChannelListener<T, P, V>
{
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.async_clone().await);
        let vault = self.vault.async_clone().await;
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
        let trust_policy = self.trust_policy.async_clone().await;
        let profile = self.profile.async_clone().await;
        SecureChannelWorker::create_responder(
            ctx,
            profile,
            trust_policy,
            self.listener_address.clone(),
            msg,
        )
        .await
    }
}
