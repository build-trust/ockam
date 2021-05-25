use crate::{ProfileTrait, Responder, TrustPolicy};
use ockam_channel::{CreateResponderChannelMessage, SecureChannel};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;
use rand::random;

pub(crate) struct ProfileChannelListener<T: TrustPolicy, P: ProfileTrait, V: XXVault> {
    trust_policy: T,
    profile: P,
    vault: V,
    listener_address: Option<Address>,
}

impl<T: TrustPolicy, P: ProfileTrait, V: XXVault> ProfileChannelListener<T, P, V> {
    pub fn new(trust_policy: T, profile: P, vault: V) -> Self {
        ProfileChannelListener {
            trust_policy,
            profile,
            vault,
            listener_address: None,
        }
    }
}

#[ockam_core::worker]
impl<T: TrustPolicy, P: ProfileTrait, V: XXVault> Worker for ProfileChannelListener<T, P, V> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let listener_address: Address = random();
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.clone());
        let vault = self.vault.clone();
        SecureChannel::create_listener_extended(
            ctx,
            listener_address.clone(),
            new_key_exchanger,
            vault,
        )
        .await?;

        self.listener_address = Some(listener_address);

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.stop_worker(self.listener_address.take().unwrap()).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let trust_policy = self.trust_policy.clone();
        Responder::create(
            ctx,
            &mut self.profile,
            trust_policy,
            self.listener_address.clone().unwrap(),
            msg,
        )
        .await
    }
}
