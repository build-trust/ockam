use crate::credential::Credential;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::finalizer::Finalizer;
use crate::secure_channel::initiator_state_machine::Action::SendMessage;
use crate::secure_channel::initiator_state_machine::Event::{Initialize, ReceivedMessage};
use crate::secure_channel::initiator_state_machine::InitiatorStateMachine;
use crate::secure_channel::initiator_state_machine::InitiatorStatus::Ready;
use crate::secure_channel::{Addresses, Role};
use crate::{to_xx_vault, Identity, SecureChannels, TrustContext, TrustPolicy};
use alloc::sync::Arc;
use core::time::Duration;
use ockam_core::{
    Address, AllowAll, Any, DenyAll, LocalOnwardOnly, LocalSourceOnly, Mailbox, Mailboxes,
    OutgoingAccessControl, Route, Routed,
};
use ockam_core::{Result, Worker};
use ockam_node::callback::CallbackSender;
use ockam_node::{Context, WorkerBuilder};
use tracing::debug;

pub(crate) struct InitiatorWorker {
    secure_channels: Arc<SecureChannels>,
    callback_sender: CallbackSender<()>,
    state_machine: InitiatorStateMachine,
    identity: Identity,
    remote_route: Route,
    addresses: Addresses,
    decryptor: Option<DecryptorWorker>,
}

#[ockam_core::worker]
impl Worker for InitiatorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> Result<()> {
        let SendMessage(message) = self.state_machine.on_event(Initialize).await?;
        context
            .send_from_address(
                self.remote_route.clone(),
                message,
                self.addresses.decryptor_remote.clone(),
            )
            .await
    }

    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        if let Some(decryptor) = self.decryptor.as_mut() {
            // since we cannot replace this worker with the decryptor worker we act as a relay instead
            return decryptor.handle_message(context, message).await;
        };

        // we only set it once to avoid redirects attack
        self.remote_route = message.return_route();
        let SendMessage(message) = self
            .state_machine
            .on_event(ReceivedMessage(message.into_transport_message().payload))
            .await?;
        context
            .send_from_address(
                self.remote_route.clone(),
                message,
                self.addresses.decryptor_remote.clone(),
            )
            .await?;

        match self.state_machine.state.status.clone() {
            Ready {
                their_identity,
                keys,
            } => {
                let finalizer = Finalizer {
                    secure_channels: self.secure_channels.clone(),
                    identity: self.identity.clone(),
                    their_identity,
                    keys,
                    addresses: self.addresses.clone(),
                    remote_route: self.remote_route.clone(),
                };

                self.decryptor = Some(finalizer.finalize(context, Role::Initiator).await?);
                self.callback_sender.send(()).await?;
            }
            _ => (),
        };
        Ok(())
    }
}

impl InitiatorWorker {
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
        remote_route: Route,
        timeout: Duration,
    ) -> Result<Address> {
        let vault = to_xx_vault(secure_channels.vault());
        let identities = secure_channels.identities();
        let state_machine = InitiatorStateMachine::new(
            vault,
            identities,
            identity.clone(),
            credentials.clone(),
            trust_policy.clone(),
            trust_context.clone(),
        );

        let (mut callback_waiter, callback_sender) = ockam_node::callback::new_callback();

        let worker = Self {
            secure_channels,
            callback_sender,
            state_machine,
            identity: identity.clone(),
            remote_route: remote_route.clone(),
            addresses: addresses.clone(),
            decryptor: None,
        };

        WorkerBuilder::with_mailboxes(
            Self::create_mailboxes(&addresses, decryptor_outgoing_access_control),
            worker,
        )
        .start(context)
        .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();
        debug!(
            "Starting SecureChannel Initiator at remote: {}",
            &decryptor_remote
        );

        callback_waiter.receive_timeout(timeout).await?;

        Ok(addresses.encryptor)
    }

    pub(crate) fn create_mailboxes(
        addresses: &Addresses,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Mailboxes {
        let remote_mailbox = Mailbox::new(
            addresses.decryptor_remote.clone(),
            // Doesn't matter since we check incoming messages cryptographically,
            // but this may be reduced to allowing only from the transport connection that was used
            // to create this channel initially
            Arc::new(AllowAll),
            // Communicate to the other side of the channel during key exchange
            Arc::new(AllowAll),
        );
        let internal_mailbox = Mailbox::new(
            addresses.decryptor_internal.clone(),
            Arc::new(DenyAll),
            decryptor_outgoing_access_control,
        );
        let api_mailbox = Mailbox::new(
            addresses.decryptor_api.clone(),
            Arc::new(LocalSourceOnly),
            Arc::new(LocalOnwardOnly),
        );

        Mailboxes::new(remote_mailbox, vec![internal_mailbox, api_mailbox])
    }
}
