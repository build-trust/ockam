use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;
use core::time::Duration;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    AllowAll, Any, Decodable, DenyAll, Error, Mailbox, Mailboxes, OutgoingAccessControl, Route,
    Routed,
};
use ockam_core::{AllowOnwardAddress, Result, Worker};
use ockam_node::callback::CallbackSender;
use ockam_node::{Context, WorkerBuilder};
use tracing::{debug, info};

use crate::models::{CredentialAndPurposeKey, CredentialData, Identifier, VersionedData};
use crate::secure_channel::decryptor::DecryptorHandler;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::handshake::handshake_state_machine::Action::SendMessage;
use crate::secure_channel::handshake::handshake_state_machine::Event::{
    Initialize, ReceivedMessage,
};
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, HandshakeResults, StateMachine,
};
use crate::secure_channel::handshake::initiator_state_machine::InitiatorStateMachine;
use crate::secure_channel::handshake::responder_state_machine::ResponderStateMachine;
use crate::secure_channel::{Addresses, Role};
use crate::{
    ChangeHistoryRepository, IdentityError, SecureChannelPurposeKey, SecureChannelRegistryEntry,
    SecureChannels, TimestampInSeconds, TrustContext, TrustPolicy,
};

/// This struct implements a Worker receiving and sending messages
/// on one side of the secure channel creation as specified with its role: INITIATOR or RESPONDER
pub(crate) struct HandshakeWorker {
    secure_channels: Arc<SecureChannels>,
    callback_sender: Option<CallbackSender<()>>,
    state_machine: Box<dyn StateMachine>,
    identifier: Identifier,
    addresses: Addresses,
    role: Role,
    remote_route: Option<Route>,
    decryptor_handler: Option<DecryptorHandler>,
    min_credential_refresh_interval: Duration,
    min_credential_expiration: Option<TimestampInSeconds>,
    refresh_credential_time_gap: Duration,
    trust_context: Option<TrustContext>,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,
    should_send_close: Arc<AtomicBool>,
}

#[ockam_core::worker]
impl Worker for HandshakeWorker {
    type Message = Any;
    type Context = Context;

    /// Initialize the state machine with an `Initialize` event
    /// Depending on the state machine role there might be a message to send to the other party
    async fn initialize(&mut self, context: &mut Self::Context) -> Result<()> {
        match self.state_machine.on_event(Initialize).await? {
            SendMessage(message) => {
                debug!(
                    "remote route {:?}, decryptor remote {:?}",
                    self.remote_route.clone(),
                    self.addresses.decryptor_remote.clone()
                );
                context
                    .send_from_address(
                        self.remote_route()?,
                        message,
                        self.addresses.decryptor_remote.clone(),
                    )
                    .await
            }
            Action::NoAction => Ok(()),
        }
    }

    /// Handle a message coming from the other party
    /// If the handshake has been fully performed then we can delegate this message to the
    /// secure channel Decryptor.
    /// Otherwise we unpack the message payload and send it to the state machine to trigger
    /// a transition
    async fn handle_message(
        &mut self,
        context: &mut Self::Context,
        message: Routed<Self::Message>,
    ) -> Result<()> {
        // Once the decryptor has been initialized, let it handle messages
        // Some messages can come from other systems using the remote address
        // and some messages can come from the current node when the decryptor
        // used to support the decryption of Kafka messages for example
        if let Some(decryptor_handler) = self.decryptor_handler.as_mut() {
            let msg_addr = message.msg_addr();

            let result = if msg_addr == self.addresses.decryptor_remote {
                decryptor_handler.handle_decrypt(context, message).await
            } else if msg_addr == self.addresses.decryptor_api {
                decryptor_handler.handle_decrypt_api(context, message).await
            } else {
                Err(IdentityError::UnknownChannelMsgDestination)?
            };
            return result;
        };

        let transport_message = message.into_transport_message();
        if let SendMessage(message) = self
            .state_machine
            .on_event(ReceivedMessage(Vec::<u8>::decode(
                &transport_message.payload,
            )?))
            .await?
        {
            // set the remote route by taking the most up to date message return route
            // In the case of the initiator the first return route mentions the secure channel listener
            // address so we need to wait for the return route corresponding to the remote handshake worker
            // when it has been spawned
            self.remote_route = Some(transport_message.return_route);

            context
                .send_from_address(
                    self.remote_route()?,
                    message,
                    self.addresses.decryptor_remote.clone(),
                )
                .await?
        };

        // if we reached the final state we can make a pair of encryptor/decryptor
        if let Some(final_state) = self.state_machine.get_handshake_results() {
            // start the encryptor worker and return the decryptor
            self.decryptor_handler = Some(self.finalize(context, final_state).await?);
            if let Some(callback_sender) = self.callback_sender.take() {
                callback_sender.send(())?;
            }
        };

        Ok(())
    }

    async fn shutdown(&mut self, context: &mut Self::Context) -> Result<()> {
        let _ = context.stop_worker(self.addresses.encryptor.clone()).await;
        self.secure_channels
            .secure_channel_registry
            .unregister_channel(&self.addresses.encryptor);

        if let Some(handler) = &self.decryptor_handler {
            handler.shutdown().await?
        }

        Ok(())
    }
}

impl HandshakeWorker {
    /// Create a new HandshakeWorker with a role of either INITIATOR or RESPONDER
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create(
        context: &Context,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        identifier: Identifier,
        purpose_key: SecureChannelPurposeKey,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credentials: Vec<CredentialAndPurposeKey>,
        min_credential_refresh_interval: Duration,
        refresh_credential_time_gap: Duration,
        trust_context: Option<TrustContext>,
        remote_route: Option<Route>,
        timeout: Option<Duration>,
        role: Role,
    ) -> Result<()> {
        let vault = secure_channels.identities.vault().secure_channel_vault;
        let identities = secure_channels.identities();

        let min_credential_expiration = credentials
            .iter()
            .filter_map(|credential| {
                minicbor::decode::<VersionedData>(&credential.credential.data).ok()
            })
            .filter_map(|data| CredentialData::get_data(&data).ok())
            .map(|data| data.expires_at)
            .min();

        let state_machine: Box<dyn StateMachine> = if role.is_initiator() {
            Box::new(
                InitiatorStateMachine::new(
                    vault,
                    identities.clone(),
                    identifier.clone(),
                    purpose_key,
                    credentials,
                    trust_policy,
                    trust_context.clone(),
                )
                .await?,
            )
        } else {
            Box::new(
                ResponderStateMachine::new(
                    vault,
                    identities.clone(),
                    identifier.clone(),
                    purpose_key,
                    credentials,
                    trust_policy,
                    trust_context.clone(),
                )
                .await?,
            )
        };

        let (callback_waiter, callback_sender) = if role.is_initiator() {
            let callback = ockam_node::callback::new_callback();
            (Some(callback.0), Some(callback.1))
        } else {
            (None, None)
        };

        let worker = Self {
            secure_channels,
            callback_sender,
            state_machine,
            identifier,
            role,
            remote_route: remote_route.clone(),
            addresses: addresses.clone(),
            decryptor_handler: None,
            min_credential_refresh_interval,
            min_credential_expiration,
            refresh_credential_time_gap,
            trust_context,
            change_history_repository: identities.change_history_repository(),
            should_send_close: Arc::new(AtomicBool::new(true)),
        };

        WorkerBuilder::new(worker)
            .with_mailboxes(Self::create_mailboxes(
                &addresses,
                decryptor_outgoing_access_control,
            ))
            .start(context)
            .await?;

        let decryptor_remote = addresses.decryptor_remote.clone();
        debug!(
            "Starting SecureChannel {} at remote: {}",
            role, &decryptor_remote
        );

        // before sending messages make sure that the handshake is finished and
        // the encryptor worker is ready
        if role.is_initiator() {
            if let Some(callback_waiter) = callback_waiter {
                // wait until the handshake is finished
                if let Some(timeout) = timeout {
                    callback_waiter.receive_timeout(timeout).await?;
                } else {
                    callback_waiter.receive().await?;
                }
            }
        }

        Ok(())
    }

    /// Return the route for the other party's handshake worker
    fn remote_route(&self) -> Result<Route> {
        self.remote_route.clone().ok_or_else(|| {
            Error::new(
                Origin::KeyExchange,
                Kind::Invalid,
                "a remote route should have been already set",
            )
        })
    }

    /// Create mailboxes and access rights for the workers involved in the secure channel creation
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
            Arc::new(AllowAll),
            Arc::new(AllowAll),
        );

        Mailboxes::new(remote_mailbox, vec![internal_mailbox, api_mailbox])
    }

    /// Finalize the handshake by creating a `Decryptor` and an `EncryptorWorker`
    /// Note that `EncryptorWorker` is actually started as an independent worker while
    /// the `Decryptor` is directly used by this worker to delegate the decryption of messages
    async fn finalize(
        &self,
        context: &Context,
        handshake_results: HandshakeResults,
    ) -> Result<DecryptorHandler> {
        // create a decryptor to delegate the processing of all messages after the handshake
        let decryptor = DecryptorHandler::new(
            self.secure_channels.identities.clone(),
            self.trust_context.clone(),
            self.role.str(),
            self.addresses.clone(),
            handshake_results.handshake_keys.decryption_key,
            self.secure_channels.identities.vault().secure_channel_vault,
            handshake_results.their_identifier.clone(),
            self.should_send_close.clone(),
        );

        // create a separate encryptor worker which will be started independently
        {
            let encryptor = EncryptorWorker::new(
                self.role.str(),
                self.addresses.clone(),
                self.remote_route()?,
                Encryptor::new(
                    handshake_results.handshake_keys.encryption_key,
                    0,
                    self.secure_channels.identities.vault().secure_channel_vault,
                ),
                self.identifier.clone(),
                self.change_history_repository.clone(),
                self.min_credential_expiration,
                self.min_credential_refresh_interval,
                self.refresh_credential_time_gap,
                self.trust_context.clone(),
                self.should_send_close.clone(),
            );

            let next_hop = self.remote_route()?.next()?.clone();
            let main_mailbox = Mailbox::new(
                self.addresses.encryptor.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowOnwardAddress(next_hop)),
            );
            let api_mailbox = Mailbox::new(
                self.addresses.encryptor_api.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            );
            let internal_mailbox = Mailbox::new(
                self.addresses.encryptor_internal.clone(),
                Arc::new(AllowAll),
                Arc::new(DenyAll),
            );

            WorkerBuilder::new(encryptor)
                .with_mailboxes(Mailboxes::new(
                    main_mailbox,
                    vec![api_mailbox, internal_mailbox],
                ))
                .start(context)
                .await?;
        }

        info!(
            "Initialized SecureChannel {} at local: {}, remote: {}",
            self.role.str(),
            &self.addresses.encryptor,
            &self.addresses.decryptor_remote
        );

        let their_decryptor_address = self
            .remote_route()?
            .iter()
            .last()
            .expect("the remote route should not be empty")
            .clone();

        let info = SecureChannelRegistryEntry::new(
            self.addresses.encryptor.clone(),
            self.addresses.encryptor_api.clone(),
            self.addresses.decryptor_remote.clone(),
            self.addresses.decryptor_api.clone(),
            self.role.is_initiator(),
            self.identifier.clone(),
            handshake_results.their_identifier,
            their_decryptor_address,
        );

        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }
}
