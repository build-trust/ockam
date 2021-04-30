use crate::{async_worker, Profile, ProfileVault, Responder, XXNewKeyExchanger};
use ockam_channel::{CreateResponderChannelMessage, SecureChannel};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;
use rand::random;

pub(crate) struct ProfileChannelListener<V: ProfileVault> {
    profile: Profile<V>, // TODO: Avoid copying profile
    vault: V,
    listener_address: Option<Address>,
}

impl<V: ProfileVault> ProfileChannelListener<V> {
    pub fn new(profile: Profile<V>, vault: V) -> Self {
        ProfileChannelListener {
            profile,
            vault,
            listener_address: None,
        }
    }
}

#[async_worker]
impl<V: ProfileVault> Worker for ProfileChannelListener<V> {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let listener_address: Address = random();
        let new_key_exchanger = XXNewKeyExchanger::new(self.vault.clone());
        let vault = self.vault.clone();
        SecureChannel::create_listener(ctx, listener_address.clone(), new_key_exchanger, vault)
            .await?;

        self.listener_address = Some(listener_address);

        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
        // TODO: ctx.stop_worker(self.listener_address.take().unwrap()).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        Responder::create(
            ctx,
            &mut self.profile,
            self.listener_address.clone().unwrap(),
            msg,
        )
        .await
    }
}
