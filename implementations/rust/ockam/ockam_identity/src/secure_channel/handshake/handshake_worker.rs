use crate::credential::Credential;
use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::decryptor_worker::DecryptorWorker;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::handshake::handshake_state::HandshakeResults;
use crate::secure_channel::handshake::handshake_state_machine::Action::SendMessage;
use crate::secure_channel::handshake::handshake_state_machine::Event::{
    Initialize, ReceivedMessage,
};
use crate::secure_channel::handshake::handshake_state_machine::{Action, StateMachine};
use crate::secure_channel::handshake::initiator_state_machine::InitiatorStateMachine;
use crate::secure_channel::handshake::responder_state_machine::ResponderStateMachine;
use crate::secure_channel::{Addresses, Role};
use crate::{
    to_xx_initialized, to_xx_vault, Identity, SecureChannelRegistryEntry, SecureChannels,
    TrustContext, TrustPolicy,
};
use alloc::sync::Arc;
use core::time::Duration;
use ockam_core::{
    Address, AllowAll, Any, DenyAll, LocalOnwardOnly, LocalSourceOnly, Mailbox, Mailboxes,
    OutgoingAccessControl, Route, Routed,
};
use ockam_core::{AllowOnwardAddress, Result, Worker};
use ockam_node::callback::CallbackSender;
use ockam_node::{Context, WorkerBuilder};
use tracing::{debug, info};

pub(crate) struct HandshakeWorker {
    secure_channels: Arc<SecureChannels>,
    callback_sender: CallbackSender<()>,
    state_machine: Box<dyn StateMachine>,
    identity: Identity,
    addresses: Addresses,
    role: Role,
    remote_route: Option<Route>,
    decryptor: Option<DecryptorWorker>,
}

#[ockam_core::worker]
impl Worker for HandshakeWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, context: &mut Self::Context) -> Result<()> {
        match self.state_machine.on_event(Initialize).await? {
            SendMessage(message) => {
                context
                    .send_from_address(
                        self.remote_route(),
                        message,
                        self.addresses.decryptor_remote.clone(),
                    )
                    .await
            }
            Action::NoAction => Ok(()),
        }
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
        self.remote_route = Some(message.return_route());
        match self
            .state_machine
            .on_event(ReceivedMessage(message.into_transport_message().payload))
            .await?
        {
            SendMessage(message) => {
                context
                    .send_from_address(
                        self.remote_route(),
                        message,
                        self.addresses.decryptor_remote.clone(),
                    )
                    .await?
            }
            _ => (),
        }

        if let Some(final_state) = self.state_machine.get_handshake_results() {
            // start the encryptor worker and return the decryptor
            self.decryptor = Some(self.finalize(context, final_state).await?);
            self.callback_sender.send(()).await?;
        };
        Ok(())
    }
}

impl HandshakeWorker {
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
        remote_route: Option<Route>,
        timeout: Option<Duration>,
        role: Role,
    ) -> Result<Address> {
        let vault = to_xx_vault(secure_channels.vault());
        let identities = secure_channels.identities();
        let state_machine: Box<dyn StateMachine> = if role.is_initiator() {
            Box::new(
                InitiatorStateMachine::new(
                    vault,
                    identities,
                    identity.clone(),
                    credentials.clone(),
                    trust_policy.clone(),
                    trust_context.clone(),
                )
                .await?,
            )
        } else {
            Box::new(
                ResponderStateMachine::new(
                    vault,
                    identities,
                    identity.clone(),
                    credentials.clone(),
                    trust_policy.clone(),
                    trust_context.clone(),
                )
                .await?,
            )
        };

        let (mut callback_waiter, callback_sender) = ockam_node::callback::new_callback();

        let worker = Self {
            secure_channels,
            callback_sender,
            state_machine,
            identity: identity.clone(),
            role: role.clone(),
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

        if role.is_initiator() {
            if let Some(timeout) = timeout {
                callback_waiter.receive_timeout(timeout).await?;
            }
        } else {
            // wait until the worker is ready to receive messages
            context.wait_for(addresses.decryptor_remote.clone()).await?;
        }

        Ok(addresses.encryptor)
    }

    fn remote_route(&self) -> Route {
        self.remote_route
            .clone()
            .expect("a remote route must have been already set for sending a message")
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

    async fn finalize(
        &self,
        context: &Context,
        handshake_results: HandshakeResults,
    ) -> Result<DecryptorWorker> {
        // decryptor worker
        let decryptor = DecryptorWorker::new(
            self.role.str(),
            self.addresses.clone(),
            Decryptor::new(
                handshake_results.decryption_key,
                to_xx_initialized(self.secure_channels.identities.vault()),
            ),
            handshake_results.their_identity.identifier(),
        );

        // encryptor worker
        {
            let encryptor = EncryptorWorker::new(
                self.role.str(),
                self.addresses.clone(),
                self.remote_route(),
                Encryptor::new(
                    handshake_results.encryption_key,
                    0,
                    to_xx_initialized(self.secure_channels.identities.vault()),
                ),
            );

            let next_hop = self.remote_route().next()?.clone();
            let main_mailbox = Mailbox::new(
                self.addresses.encryptor.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowOnwardAddress(next_hop)),
            );
            let api_mailbox = Mailbox::new(
                self.addresses.encryptor_api.clone(),
                Arc::new(LocalSourceOnly),
                Arc::new(LocalOnwardOnly),
            );

            WorkerBuilder::with_mailboxes(
                Mailboxes::new(main_mailbox, vec![api_mailbox]),
                encryptor,
            )
            .start(context)
            .await?;
        }

        info!(
            "Initialized SecureChannel {} at local: {}, remote: {}",
            self.role.str(),
            &self.addresses.encryptor,
            &self.addresses.decryptor_remote
        );

        let info = SecureChannelRegistryEntry::new(
            self.addresses.encryptor.clone(),
            self.addresses.encryptor_api.clone(),
            self.addresses.decryptor_remote.clone(),
            self.addresses.decryptor_api.clone(),
            self.role.is_initiator(),
            self.identity.identifier(),
            handshake_results.their_identity.identifier(),
        );

        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }
}
