use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Any, Result, Routed, Worker};
use ockam_node::Context;

use crate::models::Identifier;
use crate::secure_channel::addresses::Addresses;
use crate::secure_channel::encryptor_worker::RemoteRoute;
use crate::secure_channel::handshake_worker::HandshakeWorker;
use crate::secure_channel::options::SecureChannelListenerOptions;
use crate::secure_channel::role::Role;
use crate::secure_channels::secure_channels::SecureChannels;
use crate::SecureChannelRepository;

pub(crate) struct SecureChannelListenerWorker {
    secure_channels: Arc<SecureChannels>,
    identifier: Identifier,
    options: SecureChannelListenerOptions,
    secure_channel_repository: Option<Arc<dyn SecureChannelRepository>>,
}

impl SecureChannelListenerWorker {
    fn new(
        secure_channels: Arc<SecureChannels>,
        identifier: Identifier,
        options: SecureChannelListenerOptions,
    ) -> Self {
        let secure_channel_repository = if options.is_persistent {
            Some(secure_channels.secure_channel_repository())
        } else {
            None
        };

        Self {
            secure_channels,
            identifier,
            options,
            secure_channel_repository,
        }
    }

    pub fn create(
        ctx: &Context,
        secure_channels: Arc<SecureChannels>,
        identifier: &Identifier,
        address: Address,
        options: SecureChannelListenerOptions,
    ) -> Result<()> {
        options.setup_flow_control_for_listener(ctx.flow_controls(), &address);

        let listener = Self::new(secure_channels.clone(), identifier.clone(), options);

        // FIXME: add ABAC policies for the key_exchange_only listener?
        ctx.start_worker(address, listener)?;

        Ok(())
    }
}

#[ockam_core::worker]
impl Worker for SecureChannelListenerWorker {
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
            ctx.primary_address(),
            &addresses,
        );
        let decryptor_outgoing_access_control = self
            .options
            .create_decryptor_outgoing_access_control(ctx.flow_controls(), flow_control_id);

        // TODO: Allow manual PurposeKey management
        let purpose_key = self
            .secure_channels
            .identities
            .purpose_keys()
            .purpose_keys_creation()
            .get_or_create_secure_channel_purpose_key(&self.identifier)
            .await?;

        let credential_retriever = match &self.options.credential_retriever_creator {
            Some(credential_retriever_creator) => {
                // Only create, initialization should not happen here to avoid blocking listener
                let credential_retriever = credential_retriever_creator
                    .create(&self.identifier)
                    .await?;
                Some(credential_retriever)
            }
            None => None,
        };

        HandshakeWorker::create(
            ctx,
            self.secure_channels.clone(),
            addresses.clone(),
            self.identifier.clone(),
            purpose_key,
            self.options.trust_policy.clone(),
            decryptor_outgoing_access_control,
            credential_retriever,
            self.options.authority.clone(),
            None,
            None,
            Role::Responder,
            self.options.key_exchange_only,
            self.secure_channel_repository.clone(),
            RemoteRoute::create(),
        )
        .await?;

        let mut local_message = message.into_local_message();
        local_message = local_message.replace_front_onward_route(addresses.decryptor_remote)?;

        ctx.forward(local_message).await
    }
}
