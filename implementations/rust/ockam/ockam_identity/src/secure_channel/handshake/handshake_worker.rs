use core::sync::atomic::AtomicBool;
use core::time::Duration;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    AllowAll, Any, DenyAll, Error, Mailbox, Mailboxes, NeutralMessage, OutgoingAccessControl,
    Route, Routed, SecureChannelMetadata,
};
use ockam_core::{Result, Worker};
use ockam_node::callback::CallbackSender;
use ockam_node::{Context, WorkerBuilder};
use ockam_vault::AeadSecretKeyHandle;
use tracing::{debug, error, info, warn};
use tracing_attributes::instrument;

use crate::models::Identifier;
use crate::secure_channel::decryptor::DecryptorHandler;
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::{
    EncryptorWorker, RemoteRoute, SecureChannelSharedState,
};
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
    ChangeHistoryRepository, CredentialRetriever, IdentityError, PersistedSecureChannel,
    SecureChannelPurposeKey, SecureChannelRegistryEntry, SecureChannelRepository, SecureChannels,
    TrustPolicy,
};

/// This struct implements a Worker receiving and sending messages
/// on one side of the secure channel creation as specified with its role: INITIATOR or RESPONDER
pub(crate) struct HandshakeWorker {
    secure_channels: Arc<SecureChannels>,
    callback_sender: Option<CallbackSender<Identifier>>,
    state_machine: Option<Box<dyn StateMachine>>,
    my_identifier: Identifier,
    addresses: Addresses,
    role: Role,
    key_exchange_only: bool,
    remote_route: Option<Route>,
    decryptor_handler: Option<DecryptorHandler>,

    authority: Option<Identifier>,
    change_history_repository: Arc<dyn ChangeHistoryRepository>,

    credential_retriever: Option<Arc<dyn CredentialRetriever>>,

    secure_channel_repository: Option<Arc<dyn SecureChannelRepository>>,

    shared_state: SecureChannelSharedState,
}

#[ockam_core::worker]
impl Worker for HandshakeWorker {
    type Message = Any;
    type Context = Context;

    /// Initialize the state machine with an `Initialize` event
    /// Depending on the state machine role there might be a message to send to the other party
    async fn initialize(&mut self, context: &mut Self::Context) -> Result<()> {
        if let Some(credential_retriever) = &self.credential_retriever {
            credential_retriever.initialize().await?;
        }

        if let Some(state_machine) = self.state_machine.as_mut() {
            match state_machine.on_event(Initialize).await? {
                SendMessage(message) => {
                    debug!(
                        "remote route {:?}, decryptor remote {:?}",
                        self.remote_route.clone(),
                        self.addresses.decryptor_remote.clone()
                    );
                    context
                        .send_from_address(
                            self.remote_route()?,
                            NeutralMessage::from(message),
                            self.addresses.decryptor_remote.clone(),
                        )
                        .await
                }
                Action::NoAction => Ok(()),
            }
        } else {
            Ok(())
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
        if self.decryptor_handler.is_some() {
            self.handle_decrypt(context, message).await
        } else {
            self.handle_handshake(context, message).await
        }
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
        my_identifier: Identifier,
        purpose_key: SecureChannelPurposeKey,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        credential_retriever: Option<Arc<dyn CredentialRetriever>>,
        authority: Option<Identifier>,
        remote_route: Option<Route>,
        timeout: Option<Duration>,
        role: Role,
        key_exchange_only: bool,
        secure_channel_repository: Option<Arc<dyn SecureChannelRepository>>,
        encryptor_remote_route: Arc<RwLock<RemoteRoute>>,
    ) -> Result<Option<Identifier>> {
        let vault = secure_channels.identities.vault().secure_channel_vault;
        let identities = secure_channels.identities();

        let state_machine: Box<dyn StateMachine> = if role.is_initiator() {
            Box::new(
                InitiatorStateMachine::new(
                    vault,
                    identities.clone(),
                    my_identifier.clone(),
                    purpose_key,
                    credential_retriever.clone(),
                    trust_policy,
                    authority.clone(),
                )
                .await?,
            )
        } else {
            Box::new(
                ResponderStateMachine::new(
                    vault,
                    identities.clone(),
                    my_identifier.clone(),
                    purpose_key,
                    credential_retriever.clone(),
                    trust_policy,
                    authority.clone(),
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

        let shared_state = SecureChannelSharedState {
            should_send_close: Arc::new(AtomicBool::new(true)),
            remote_route: encryptor_remote_route,
        };
        let worker = Self {
            secure_channels,
            callback_sender,
            state_machine: Some(state_machine),
            my_identifier: my_identifier.clone(),
            role,
            key_exchange_only,
            remote_route: remote_route.clone(),
            addresses: addresses.clone(),
            decryptor_handler: None,
            credential_retriever,
            authority,
            change_history_repository: identities.change_history_repository(),
            secure_channel_repository,
            shared_state,
        };

        WorkerBuilder::new(worker)
            .with_mailboxes(Self::create_mailboxes(
                &addresses,
                decryptor_outgoing_access_control,
            ))
            .start(context)
            .await?;

        debug!(
            "Starting SecureChannel {} at remote: {}, local: {}",
            role, addresses.decryptor_remote, addresses.encryptor
        );

        // before sending messages make sure that the handshake is finished and
        // the encryptor worker is ready
        let their_identifier = if role.is_initiator() {
            if let Some(callback_waiter) = callback_waiter {
                // wait until the handshake is finished
                if let Some(timeout) = timeout {
                    let res = callback_waiter.receive_timeout(timeout).await;

                    match res {
                        Ok(their_identifier) => Some(their_identifier),
                        Err(err) => {
                            error!(
                            "Timeout {:?} or error reached when creating secure channel for: {}. Encryptor: {}. Error: {err:?}",
                            timeout, my_identifier, addresses.encryptor
                        );

                            return Err(err);
                        }
                    }
                } else {
                    Some(callback_waiter.receive().await?)
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(their_identifier)
    }

    /// This function is instrumented as handle_message to make as if there were 2 workers
    ///   - one for handshakes
    ///   - one for decryption
    ///
    /// See also the handle_decrypt method.
    #[instrument(skip_all, name = "HandshakeWorker::handle_message")]
    async fn handle_handshake(
        &mut self,
        context: &mut Context,
        message: Routed<Any>,
    ) -> Result<()> {
        let return_route = message.return_route();
        let payload = message.into_payload();

        if let SendMessage(send_message) = self
            .state_machine
            .as_mut()
            .ok_or(IdentityError::HandshakeInternalError)?
            .on_event(ReceivedMessage(payload))
            .await?
        {
            // set the remote route by taking the most up to date message return route
            // In the case of the initiator the first return route mentions the secure channel listener
            // address so we need to wait for the return route corresponding to the remote handshake worker
            // when it has been spawned
            self.remote_route = Some(return_route);

            context
                .send_from_address(
                    self.remote_route()?,
                    NeutralMessage::from(send_message),
                    self.addresses.decryptor_remote.clone(),
                )
                .await?
        };

        // if we reached the final state we can make a pair of encryptor/decryptor
        if let Some(final_state) = self
            .state_machine
            .as_ref()
            .ok_or(IdentityError::HandshakeInternalError)?
            .get_handshake_results()
        {
            // start the encryptor worker and return the decryptor
            let their_identifier = final_state.their_identifier.clone();
            self.decryptor_handler = Some(self.finalize(context, final_state).await?);
            if let Some(callback_sender) = self.callback_sender.take() {
                callback_sender.send(their_identifier)?;
            }
        };

        Ok(())
    }

    /// This function is instrumented as if there was a DecryptorWorker type for a better
    /// readability of traces (Because there's a corresponding EncryptorWorker::handle_message)
    ///
    /// In reality, there's only one worker, the HandshakeWorker, serves as both a worker for handshakes
    /// and for decryption.
    #[instrument(skip_all, name = "DecryptorWorker::handle_message")]
    async fn handle_decrypt(&mut self, context: &mut Context, message: Routed<Any>) -> Result<()> {
        let decryptor_handler = self.decryptor_handler.as_mut().unwrap();
        let msg_addr = message.msg_addr();

        if self.key_exchange_only {
            if msg_addr == self.addresses.decryptor_api {
                decryptor_handler.handle_decrypt_api(context, message).await
            } else {
                Err(IdentityError::UnknownChannelMsgDestination)?
            }
        } else if msg_addr == self.addresses.decryptor_remote {
            decryptor_handler.handle_decrypt(context, message).await
        } else if msg_addr == self.addresses.decryptor_api {
            decryptor_handler.handle_decrypt_api(context, message).await
        } else {
            Err(IdentityError::UnknownChannelMsgDestination)?
        }
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
        let their_identifier = handshake_results.their_identifier.clone();

        // create a decryptor to delegate the processing of all messages after the handshake
        let decryptor = DecryptorHandler::new(
            self.secure_channels.identities.clone(),
            self.authority.clone(),
            self.role,
            self.key_exchange_only,
            self.addresses.clone(),
            handshake_results.handshake_keys.decryption_key.clone(),
            self.secure_channels.identities.vault().secure_channel_vault,
            handshake_results.their_identifier.clone(),
            self.shared_state.clone(),
        );

        // create a separate encryptor worker which will be started independently
        {
            let (rekeying, credential_retriever) = if self.key_exchange_only {
                // only the initial exchange is needed for key exchange only
                (false, None)
            } else {
                (true, self.credential_retriever.clone())
            };

            self.shared_state.remote_route.write().unwrap().route = self.remote_route()?;
            let encryptor = EncryptorWorker::new(
                self.role.str(),
                self.key_exchange_only,
                self.addresses.clone(),
                Encryptor::new(
                    handshake_results.handshake_keys.encryption_key,
                    0.into(),
                    self.secure_channels.identities.vault().secure_channel_vault,
                    rekeying,
                ),
                self.my_identifier.clone(),
                self.change_history_repository.clone(),
                credential_retriever,
                handshake_results.presented_credential,
                self.shared_state.clone(),
            );

            let main_mailbox = Mailbox::new(
                self.addresses.encryptor.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowAll),
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
                .terminal_with_attributes(
                    self.addresses.encryptor.clone(),
                    vec![SecureChannelMetadata::attribute(
                        their_identifier.clone().into(),
                    )],
                )
                .start(context)
                .await?;
        }

        self.persist(
            their_identifier,
            &handshake_results.handshake_keys.decryption_key,
        )
        .await;

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
            self.my_identifier.clone(),
            handshake_results.their_identifier,
            their_decryptor_address,
        );

        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        Ok(decryptor)
    }

    async fn persist(&self, their_identifier: Identifier, decryption_key: &AeadSecretKeyHandle) {
        let Some(repository) = &self.secure_channel_repository else {
            info!(
                "Skipping persistence. Local: {}, Remote: {}",
                self.addresses.encryptor, &self.addresses.decryptor_remote
            );
            return;
        };

        let sc = PersistedSecureChannel::new(
            self.role,
            self.my_identifier.clone(),
            their_identifier,
            self.addresses.decryptor_remote.clone(),
            self.addresses.decryptor_api.clone(),
            decryption_key.clone(),
        );
        match repository.put(sc).await {
            Ok(_) => {
                info!(
                    "Successfully persisted secure channel. Local: {}, Remote: {}",
                    self.addresses.encryptor, &self.addresses.decryptor_remote,
                );
            }
            Err(err) => {
                warn!(
                    "Error while persisting secure channel: {err}. Local: {}, Remote: {}",
                    self.addresses.encryptor, &self.addresses.decryptor_remote
                );

                return;
            }
        }

        if let Err(err) = self
            .secure_channels
            .identities
            .vault()
            .secure_channel_vault
            .persist_aead_key(decryption_key)
            .await
        {
            warn!(
                "Error persisting secure channel key: {err}. Local: {}, Remote: {}",
                self.addresses.encryptor, &self.addresses.decryptor_remote
            );
        };
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        secure_channels: Arc<SecureChannels>,
        callback_sender: Option<CallbackSender<Identifier>>,
        state_machine: Option<Box<dyn StateMachine>>,
        my_identifier: Identifier,
        addresses: Addresses,
        role: Role,
        key_exchange_only: bool,
        remote_route: Option<Route>,
        decryptor_handler: Option<DecryptorHandler>,
        authority: Option<Identifier>,
        change_history_repository: Arc<dyn ChangeHistoryRepository>,
        credential_retriever: Option<Arc<dyn CredentialRetriever>>,
        secure_channel_repository: Option<Arc<dyn SecureChannelRepository>>,
        shared_state: SecureChannelSharedState,
    ) -> Self {
        Self {
            secure_channels,
            callback_sender,
            state_machine,
            my_identifier,
            addresses,
            role,
            key_exchange_only,
            remote_route,
            decryptor_handler,
            authority,
            change_history_repository,
            credential_retriever,
            secure_channel_repository,
            shared_state,
        }
    }
}
