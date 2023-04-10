use crate::channel::addresses::Addresses;
use crate::channel::common::CreateResponderChannelMessage;
use crate::channel::decryptor_worker::DecryptorWorker;
use crate::channel::Role;
use crate::{Identity, SecureChannelListenerOptions};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, AllowAll, AsyncTryClone, DenyAll, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener {
    options: SecureChannelListenerOptions,
    identity: Identity,
}

impl IdentityChannelListener {
    pub fn new(options: SecureChannelListenerOptions, identity: Identity) -> Self {
        Self { options, identity }
    }

    pub async fn create(
        ctx: &Context,
        address: Address,
        options: SecureChannelListenerOptions,
        identity: Identity,
    ) -> Result<()> {
        if let Some(ciphertext_flow_control) = &options.consumer_flow_control {
            if let Some(info) = &ciphertext_flow_control.info {
                ciphertext_flow_control.flow_controls.add_consumer(
                    &address,
                    &info.flow_control_id,
                    info.flow_control_policy,
                );
            }
        }

        if let Some((flow_controls, flow_control_id)) = &options.channels_producer_flow_control {
            flow_controls.add_spawner(&address, flow_control_id);
        }

        let listener = Self::new(options, identity);

        ctx.start_worker(
            address, listener, AllowAll, // TODO: @ac allow to customize
            DenyAll,
        )
        .await?;

        Ok(())
    }
}

#[ockam_core::worker]
impl Worker for IdentityChannelListener {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let identity = self.identity.async_try_clone().await?;

        // Check if the Worker that send us this message is a Producer
        // If yes - decryptor will be added to that flow_control to be able to receive further messages
        // from that Producer
        let flow_control_id =
            if let Some(ciphertext_flow_control) = &self.options.consumer_flow_control {
                ciphertext_flow_control
                    .flow_controls
                    .get_flow_control_with_producer(&msg.src_addr())
                    .map(|x| x.flow_control_id().clone())
            } else {
                None
            };

        let addresses = Addresses::generate(Role::Responder);
        let flow_control_id = self
            .options
            .setup_flow_control(&addresses, flow_control_id)?;
        let access_control = self
            .options
            .create_access_control(flow_control_id.clone())?;

        DecryptorWorker::create_responder(
            ctx,
            identity,
            addresses,
            self.options.trust_policy.clone(),
            access_control.decryptor_outgoing_access_control,
            msg,
            self.options.credential.is_some(),
            self.options.credential.clone(),
            self.authorities.clone(),
        )
        .await
    }
}
