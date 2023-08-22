use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, Any, Result, Routed, Worker};
use ockam_node::Context;

use super::super::models::{CredentialAndPurposeKey, Identifier};
use super::super::secure_channel::addresses::Addresses;
use super::super::secure_channel::handshake_worker::HandshakeWorker;
use super::super::secure_channel::options::SecureChannelListenerOptions;
use super::super::secure_channel::role::Role;
use super::super::secure_channels::secure_channels::SecureChannels;
use super::super::Purpose;

pub(crate) struct IdentityChannelListener {
    secure_channels: Arc<SecureChannels>,
    identifier: Identifier,
    options: SecureChannelListenerOptions,
}

impl IdentityChannelListener {
    fn new(
        secure_channels: Arc<SecureChannels>,
        identifier: Identifier,
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
        identifier: &Identifier,
        address: Address,
        options: SecureChannelListenerOptions,
    ) -> Result<()> {
        options.setup_flow_control_for_listener(ctx.flow_controls(), &address);

        let listener = Self::new(secure_channels.clone(), identifier.clone(), options);

        ctx.start_worker(address, listener).await?;

        Ok(())
    }

    /// If credentials are not provided via list in options
    /// get them from the trust context
    async fn get_credentials(&self, ctx: &mut Context) -> Result<Vec<CredentialAndPurposeKey>> {
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
        Ok(credentials)
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

        let credentials = self.get_credentials(ctx).await?;

        // TODO: Allow manual PurposeKey management
        let purpose_key = self
            .secure_channels
            .purpose_keys
            .get_or_create_purpose_key(&self.identifier, Purpose::SecureChannel)
            .await?;

        HandshakeWorker::create(
            ctx,
            self.secure_channels.clone(),
            addresses.clone(),
            self.identifier.clone(),
            purpose_key,
            self.options.trust_policy.clone(),
            access_control.decryptor_outgoing_access_control,
            credentials,
            self.options.trust_context.clone(),
            None,
            None,
            Role::Responder,
        )
        .await?;

        let mut local_message = message.into_local_message();
        local_message
            .transport_mut()
            .onward_route
            .modify()
            .replace(addresses.decryptor_remote);

        ctx.forward(local_message).await
    }
}
