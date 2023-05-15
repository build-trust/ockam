use crate::credential::Credential;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::finalizer::Finalizer;
use crate::secure_channel::initiator_worker::InitiatorWorker;
use crate::secure_channel::responder_state_machine::ResponderStateMachine;
use crate::secure_channel::state_machine::Action::SendMessage;
use crate::secure_channel::state_machine::Event::ReceivedMessage;
use crate::secure_channel::Addresses;
use crate::secure_channel::Role::Responder;
use crate::{to_xx_vault, Identity, SecureChannels, TrustContext, TrustPolicy};
use ockam_core::Worker;
use ockam_core::{Address, Any, OutgoingAccessControl, Routed};
use ockam_node::{Context, WorkerBuilder};
use std::sync::Arc;
use tracing::debug;

pub(crate) struct ResponderWorker {
    state_machine: ResponderStateMachine,
    secure_channels: Arc<SecureChannels>,
    identity: Identity,
    addresses: Addresses,
    decryptor: Option<DecryptorWorker>,
}

#[ockam_core::worker]
impl Worker for ResponderWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        if let Some(decryptor) = self.decryptor.as_mut() {
            // since we cannot replace this worker with the decryptor worker we act as a relay instead
            return decryptor.handle_message(context, message).await;
        };
        let remote_route = message.return_route();

        match self
            .state_machine
            .on_event(ReceivedMessage(message.into_transport_message().payload))
            .await?
        {
            SendMessage(message) => {
                context
                    .send_from_address(
                        remote_route.clone(),
                        message,
                        self.addresses.decryptor_remote.clone(),
                    )
                    .await?
            }
            _ => (),
        }

        if let Some((their_identity, keys)) = self.state_machine.ready() {
            let finalizer = Finalizer {
                secure_channels: self.secure_channels.clone(),
                identity: self.identity.clone(),
                their_identity,
                keys,
                addresses: self.addresses.clone(),
                remote_route: remote_route,
            };

            self.decryptor = Some(finalizer.finalize(context, Responder).await?);
        };
        Ok(())
    }
}

impl ResponderWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        identity: Identity,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credentials: Vec<Credential>,
        trust_context: Option<TrustContext>,
    ) -> ockam_core::Result<Address> {
        let vault = to_xx_vault(secure_channels.vault());
        let identities = secure_channels.identities();
        let state_machine = ResponderStateMachine::new(
            vault,
            identities,
            identity.clone(),
            credentials.clone(),
            trust_policy.clone(),
            trust_context.clone(),
        );

        let worker = Self {
            secure_channels,
            state_machine,
            identity: identity.clone(),
            addresses: addresses.clone(),
            decryptor: None,
        };

        WorkerBuilder::with_mailboxes(
            InitiatorWorker::create_mailboxes(&addresses, decryptor_outgoing_access_control),
            worker,
        )
        .start(context)
        .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();
        debug!(
            "Starting SecureChannel Responder at remote: {}",
            &decryptor_remote
        );

        //wait until the worker is ready to receive messages
        context.wait_for(addresses.decryptor_remote.clone()).await?;
        Ok(addresses.decryptor_remote)
    }
}
