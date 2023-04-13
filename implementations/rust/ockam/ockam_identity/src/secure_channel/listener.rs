use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::common::Role;
use crate::secure_channel::options::SecureChannelListenerOptions;
use crate::secure_channel::responder_worker::ResponderWorker;
use crate::secure_channels::secure_channels::SecureChannels;
use crate::IdentityIdentifier;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, AllowAll, Any, LocalOnwardOnly, Result, Routed, Worker};
use ockam_node::Context;

pub(crate) struct IdentityChannelListener {
    secure_channels: Arc<SecureChannels>,
    identifier: IdentityIdentifier,
    options: SecureChannelListenerOptions,
}

impl IdentityChannelListener {
    fn new(
        secure_channels: Arc<SecureChannels>,
        identifier: IdentityIdentifier,
        options: SecureChannelListenerOptions,
    ) -> Self {
        Self {
            secure_channels,
            identifier,
            options,
        }
    }

    pub async fn create(
        ctx: &Context,
        secure_channels: Arc<SecureChannels>,
        identifier: &IdentityIdentifier,
        address: Address,
        options: SecureChannelListenerOptions,
    ) -> Result<()> {
        options.setup_flow_control_for_listener(ctx.flow_controls(), &address);

        let listener = Self::new(secure_channels.clone(), identifier.clone(), options);

        ctx.start_worker(
            address,
            listener,
            AllowAll, // TODO: @ac allow to customize
            LocalOnwardOnly,
        )
        .await?;

        Ok(())
    }
}

#[ockam_core::worker]
impl Worker for IdentityChannelListener {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        let addresses = Addresses::generate(Role::Responder);
        let flow_control_id = self.options.setup_flow_control_for_channel(
            ctx.flow_controls(),
            &addresses,
            &message.src_addr(),
        );
        let access_control = self
            .options
            .create_access_control(ctx.flow_controls(), flow_control_id);

        let credentials = if self.options.credentials.is_empty() {
            if let Some(trust_context) = &self.options.trust_context {
                vec![
                    trust_context
                        .authority()?
                        .credential(ctx, &self.identifier)
                        .await?,
                ]
            } else {
                vec![]
            }
        } else {
            self.options.credentials.clone()
        };

        let decryptor_remote_address = ResponderWorker::create(
            ctx,
            self.secure_channels.clone(),
            addresses,
            self.identifier.clone(),
            self.options.trust_policy.clone(),
            access_control.decryptor_outgoing_access_control,
            credentials,
            self.options.trust_context.clone(),
        )
        .await?;

        let mut local_message = message.into_local_message();
        local_message
            .transport_mut()
            .onward_route
            .modify()
            .replace(decryptor_remote_address);

        ctx.forward(local_message).await
    }
}
