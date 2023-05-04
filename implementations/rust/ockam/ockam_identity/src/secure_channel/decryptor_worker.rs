use core::time::Duration;

use tracing::{debug, info, warn};

use ockam_core::compat::boxed::Box;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::Signature;
use ockam_core::Result;
use ockam_core::{
    async_trait, route, Address, AllowAll, AllowOnwardAddress, AllowSourceAddress, Any, Decodable,
    DenyAll, Encodable, LocalMessage, LocalOnwardOnly, LocalSourceOnly, Mailbox, Mailboxes,
    NewKeyExchanger, OutgoingAccessControl, Route, Routed, TransportMessage, Worker,
};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::{Context, MessageReceiveOptions, WorkerBuilder};

use crate::secure_channel::decryptor::Decryptor;
use crate::secure_channel::decryptor_state::{
    IdentityExchangeState, InitializedState, KeyExchangeState, State,
};
use crate::secure_channel::encryptor::Encryptor;
use crate::secure_channel::encryptor_worker::EncryptorWorker;
use crate::secure_channel::messages::IdentityChannelMessage;
use crate::secure_channel::{
    Addresses, AuthenticationConfirmation, CreateResponderChannelMessage, Role,
};
use crate::{
    to_xx_initialized, to_xx_vault, DecryptionRequest, DecryptionResponse, IdentityError,
    IdentityIdentifier, IdentitySecureChannelLocalInfo, SecureChannelRegistryEntry,
    SecureChannelTrustInfo, SecureChannels, TrustPolicy,
};

pub(crate) struct DecryptorWorker {
    state: Option<State>,
}

impl DecryptorWorker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create_initiator(
        ctx: &Context,
        secure_channels: Arc<SecureChannels>,
        identifier: IdentityIdentifier,
        remote_route: Route,
        addresses: Addresses,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        timeout: Duration,
    ) -> Result<()> {
        let mut completion_callback_ctx = ctx
            .new_detached(
                addresses.completion_callback.clone(),
                AllowSourceAddress(addresses.decryptor_callback.clone()),
                DenyAll,
            )
            .await?;

        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(secure_channels.vault()))
            .initiator()
            .await?;

        let mailboxes = Self::mailboxes(&addresses, decryptor_outgoing_access_control);

        let worker = DecryptorWorker {
            state: Some(State::new(
                Role::Initiator,
                identifier,
                secure_channels.clone(),
                addresses.clone(),
                Box::new(key_exchanger),
                remote_route,
                trust_policy,
                None,
                None,
            )),
        };

        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        debug!(
            "Starting SecureChannel Initiator at remote: {}",
            &addresses.decryptor_remote
        );

        completion_callback_ctx
            .receive_extended::<AuthenticationConfirmation>(
                MessageReceiveOptions::new().with_timeout(timeout),
            )
            .await?;

        Ok(())
    }
}

impl DecryptorWorker {
    pub(crate) async fn create_responder(
        ctx: &Context,
        secure_channels: Arc<SecureChannels>,
        addresses: Addresses,
        identifier: IdentityIdentifier,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        msg: Routed<CreateResponderChannelMessage>,
    ) -> Result<()> {
        // Route to the decryptor on the other side
        let remote_route = msg.return_route();
        let body = msg.body();
        // This is the address of the Worker on the other end that Initiator gave us to perform further negotiations.
        // This is the remote_backwards_compatibility_address
        let remote_backwards_compatibility_address = body
            .custom_payload()
            .as_ref()
            .ok_or(IdentityError::NoCustomPayload)?;
        let remote_backwards_compatibility_address =
            Address::decode(remote_backwards_compatibility_address)?;

        let vault = to_xx_vault(secure_channels.vault());
        let key_exchanger = XXNewKeyExchanger::new(vault).responder().await?;

        let mailboxes = Self::mailboxes(&addresses, decryptor_outgoing_access_control);

        let worker = DecryptorWorker {
            state: Some(State::new(
                Role::Responder,
                identifier,
                secure_channels.clone(),
                addresses.clone(),
                Box::new(key_exchanger),
                remote_route,
                trust_policy,
                Some(remote_backwards_compatibility_address),
                Some(body.payload().to_vec()),
            )),
        };

        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        debug!(
            "Starting SecureChannel Responder at remote: {}",
            &addresses.decryptor_remote
        );

        Ok(())
    }
}

impl DecryptorWorker {
    fn mailboxes(
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
        let callback_mailbox = Mailbox::new(
            addresses.decryptor_callback.clone(),
            Arc::new(AllowAll),
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

        Mailboxes::new(
            remote_mailbox,
            vec![internal_mailbox, callback_mailbox, api_mailbox],
        )
    }
}

// Key exchange
impl KeyExchangeState {
    async fn send_key_exchange_payload(
        ctx: &mut <DecryptorWorker as Worker>::Context,
        payload: Vec<u8>,
        custom_payload: Option<Vec<u8>>,
        remote_route: Route,
        decryptor_remote: Address,
    ) -> Result<()> {
        if let Some(custom_payload) = custom_payload {
            // First message from initiator goes to the channel listener
            ctx.send_from_address(
                remote_route,
                CreateResponderChannelMessage::new(payload, Some(custom_payload)),
                decryptor_remote,
            )
            .await
        } else {
            // Other messages go to the channel worker itself
            ctx.send_from_address(remote_route, payload, decryptor_remote)
                .await
        }
    }

    async fn handle_key_exchange_msg(
        mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<State> {
        self.remote_route = msg.return_route();
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;
        self.handle_key_exchange(ctx, Some(&payload)).await
    }

    async fn handle_key_exchange(
        mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        payload: Option<&[u8]>,
    ) -> Result<State> {
        if let Some(incoming_payload) = payload {
            // Received key exchange message from remote channel, need to forward it to the local key exchange
            debug!(
                "SecureChannel received KeyExchangeRemote at {}",
                &self.addresses.decryptor_remote
            );
            let exchanger = &mut self.key_exchanger;
            let _ = exchanger.handle_response(incoming_payload).await?;
        }

        // If we'll need to generate another request
        let mut request_was_sent = false;

        // Key exchange hasn't been completed -> generate and send next request
        if !self.key_exchanger.is_complete().await? {
            request_was_sent = true;
            let payload = self.key_exchanger.generate_request(&[]).await?;

            // We should send first_responder_address only with first message from the initiator
            let custom_payload = if self.role.is_initiator() && self.initialization_run {
                Some(self.addresses.decryptor_backwards_compatibility.encode()?)
            } else {
                None
            };
            self.initialization_run = false;

            Self::send_key_exchange_payload(
                ctx,
                payload,
                custom_payload,
                self.remote_route.clone(),
                self.addresses.decryptor_remote.clone(),
            )
            .await?;
        }

        // Still not completed -> wait for the next message from the other side
        if !self.key_exchanger.is_complete().await? {
            return Ok(State::KeyExchange(self));
        }

        // Key exchange completed, proceed to Identity Exchange
        let keys = self.key_exchanger.finalize().await?;
        let vault = &self.secure_channels.vault();

        let mut identity_exchange = self.into_identity_exchange(
            Encryptor::new(
                keys.encrypt_key().clone(),
                0,
                to_xx_initialized(vault.clone()),
            ),
            Decryptor::new(keys.decrypt_key().clone(), to_xx_initialized(vault.clone())),
            *keys.h(),
        );

        if !request_was_sent {
            // Key exchange was completed by processing response, no new request was required.
            // This means that it's our turn to send our Identity
            identity_exchange.send_identity(ctx, true).await?;
            Ok(State::IdentityExchange(identity_exchange))
        } else {
            // Key exchange was completed by generating our last request.
            // This means that it's their turn to send us their Identity
            // Just wait for that message
            Ok(State::IdentityExchange(identity_exchange))
        }
    }
}

// Identity exchange
impl IdentityExchangeState {
    async fn handle_exchange_identity(
        mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<State> {
        // We received an Identity
        let their_identity_id = self.handle_incoming_identity(msg).await?;

        if !self.identity_sent {
            self.send_identity(ctx, false).await?;
        }

        // We received and sent Identity - channel is initialized
        self.complete_channel_initialization(ctx, their_identity_id)
            .await
    }

    async fn complete_channel_initialization(
        mut self,
        ctx: &mut Context,
        their_identity_id: IdentityIdentifier,
    ) -> Result<State> {
        let encryptor = self
            .encryptor
            .take()
            .ok_or(IdentityError::InvalidSecureChannelInternalState)?;

        let next_hop = self.remote_route.next()?.clone();
        let encryptor = EncryptorWorker::new(
            self.role.str(),
            self.addresses.clone(),
            self.remote_route.clone(),
            self.remote_backwards_compatibility_address
                .clone()
                .ok_or(IdentityError::InvalidSecureChannelInternalState)?,
            encryptor,
        );

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

        WorkerBuilder::with_mailboxes(Mailboxes::new(main_mailbox, vec![api_mailbox]), encryptor)
            .start(ctx)
            .await?;

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
            self.identifier.clone(),
            their_identity_id.clone(),
        );
        self.secure_channels
            .secure_channel_registry()
            .register_channel(info)?;

        if self.role.is_initiator() {
            // Notify interested worker about finished init
            ctx.send_from_address(
                route![self.addresses.completion_callback.clone()],
                AuthenticationConfirmation,
                self.addresses.decryptor_callback.clone(),
            )
            .await?;
        }

        Ok(State::Initialized(
            self.into_initialized(their_identity_id.clone()),
        ))
    }

    async fn handle_incoming_identity(&mut self, msg: Routed<Any>) -> Result<IdentityIdentifier> {
        let body = Vec::<u8>::decode(msg.payload())?;
        let body = self.decryptor.decrypt(&body).await?;
        let body = TransportMessage::decode(&body)?;
        if body.onward_route.next()? != &self.addresses.decryptor_backwards_compatibility {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }
        if self.remote_backwards_compatibility_address.is_none() {
            self.remote_backwards_compatibility_address = Some(body.return_route.recipient()?);
        }
        let body = IdentityChannelMessage::decode(&body.payload)?;

        let (identity, signature) = body.consume();
        debug!(
            "Received Authentication request {}",
            &self.addresses.decryptor_remote
        );

        let identities = self.secure_channels.identities();
        let their_identity = identities
            .identities_creation()
            .decode_identity(identity.as_slice())
            .await?;
        let their_identity_id = their_identity.identifier();

        // Verify responder posses their Identity key
        let verified = identities
            .identities_keys()
            .verify_signature(
                &their_identity,
                &Signature::new(signature),
                &self.auth_hash,
                None,
            )
            .await?;

        if !verified {
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        }

        self.secure_channels
            .identities()
            .repository()
            .update_identity(&their_identity)
            .await?;

        info!(
            "Initiator verified SecureChannel from: {}",
            their_identity_id
        );

        // Check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(their_identity_id.clone());
        let trusted = self.trust_policy.check(&trust_info).await?;
        if !trusted {
            // TODO: Shutdown? Communicate error?
            return Err(IdentityError::SecureChannelTrustCheckFailed.into());
        }
        info!(
            "Initiator checked trust policy for SecureChannel from: {}",
            their_identity_id
        );

        Ok(their_identity_id.clone())
    }

    async fn send_identity(&mut self, ctx: &mut Context, first_sender: bool) -> Result<()> {
        let identity = self
            .secure_channels
            .identities
            .identities_repository
            .get_identity(&self.identifier)
            .await?;
        // Prove we posses our Identity key
        let signature = self
            .secure_channels
            .identities()
            .identities_keys()
            .create_signature(&identity, &self.auth_hash, None)
            .await?;

        let exported = identity.export()?;
        let auth_msg = if first_sender {
            IdentityChannelMessage::Request {
                identity: exported,
                signature: signature.as_ref().to_vec(),
            }
        } else {
            IdentityChannelMessage::Response {
                identity: exported,
                signature: signature.as_ref().to_vec(),
            }
        };

        let msg = TransportMessage::v1(
            self.remote_backwards_compatibility_address
                .clone()
                .ok_or(IdentityError::InvalidSecureChannelInternalState)?,
            route![self.addresses.decryptor_backwards_compatibility.clone()],
            auth_msg.encode()?,
        );
        let data = msg.encode()?;

        let encrypted = self
            .encryptor
            .as_mut()
            .ok_or(IdentityError::InvalidSecureChannelInternalState)?
            .encrypt(&data)
            .await?;

        ctx.send_from_address(
            self.remote_route.clone(),
            encrypted,
            self.addresses.decryptor_remote.clone(),
        )
        .await?;
        debug!(
            "Sent Authentication response {}",
            &self.addresses.decryptor_remote
        );

        self.identity_sent = true;
        Ok(())
    }
}

// Decryption
impl InitializedState {
    async fn handle_decrypt_api(
        &mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt API {}",
            self.role, &self.addresses.decryptor_remote
        );

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = DecryptionRequest::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&request.0).await;

        let response = match decrypted_payload {
            Ok(payload) => DecryptionResponse::Ok(payload),
            Err(err) => DecryptionResponse::Err(err),
        };

        // Send reply to the caller
        ctx.send_from_address(return_route, response, self.addresses.decryptor_api.clone())
            .await?;

        Ok(())
    }

    async fn handle_decrypt(
        &mut self,
        ctx: &mut <DecryptorWorker as Worker>::Context,
        msg: Routed<<DecryptorWorker as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt {}",
            self.role, &self.addresses.decryptor_remote
        );

        // Decode raw payload binary
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = self.decryptor.decrypt(&payload).await?;

        // Encrypted data should be a TransportMessage
        let mut transport_message = TransportMessage::decode(&decrypted_payload)?;

        // Ensure message goes to backwards compatibility address and skip that address
        if transport_message.onward_route.step()?
            != self.addresses.decryptor_backwards_compatibility
        {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        // Add encryptor hop in the return_route (instead of our address)
        transport_message
            .return_route
            .modify()
            .prepend(self.addresses.encryptor.clone());

        // Mark message LocalInfo with IdentitySecureChannelLocalInfo,
        // replacing any pre-existing entries
        let local_info =
            IdentitySecureChannelLocalInfo::mark(vec![], self.their_identity_id.clone())?;

        let msg = LocalMessage::new(transport_message, local_info);

        match ctx
            .forward_from_address(msg, self.addresses.decryptor_internal.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    "{} forwarding decrypted message from {}",
                    err, &self.addresses.encryptor
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl Worker for DecryptorWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // Process first received message (in case of Responder),
        // generate and send the next message

        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        let new_state = match state {
            State::KeyExchange(mut state) => {
                let init_payload = match &state.role {
                    Role::Initiator => None,
                    Role::Responder => state.initial_responder_payload.take(),
                };
                state
                    .handle_key_exchange(ctx, init_payload.as_deref())
                    .await?
            }
            _ => {
                return Err(IdentityError::InvalidSecureChannelInternalState.into());
            }
        };
        self.state = Some(new_state);

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        // since once initialized, the state can't change anymore we just borrow a mutable,
        // doing so we avoid extra copies (in case the compiler don't optimize them away),
        // and we make sure the state remain initialized even in the case of an error
        if let State::Initialized(state) = self
            .state
            .as_mut()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?
        {
            if msg_addr == state.addresses.decryptor_remote {
                state.handle_decrypt(ctx, msg).await?;
            } else if msg_addr == state.addresses.decryptor_api {
                state.handle_decrypt_api(ctx, msg).await?;
            } else {
                return Err(IdentityError::UnknownChannelMsgDestination.into());
            }

            return Ok(());
        }

        let state = self
            .state
            .take()
            .ok_or_else(|| IdentityError::InvalidSecureChannelInternalState)?;

        // if any error occurs during key and identity exchange the state will become
        // invalid, this enforces atomicity reducing the possibility of attacks

        let new_state = match state {
            State::KeyExchange(state) => {
                if msg_addr == state.addresses.decryptor_remote {
                    let result = state.handle_key_exchange_msg(ctx, msg).await;
                    if result.is_err() {
                        if let Err(err) = ctx.stop_worker(msg_addr.clone()).await {
                            warn!("cannot stop decryptor: {err} using address {msg_addr}");
                        }
                    }
                    result?
                } else {
                    // let's ignore invalid address messages errors
                    self.state = Some(State::KeyExchange(state));
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::IdentityExchange(state) => {
                if msg_addr == state.addresses.decryptor_remote {
                    let result = state.handle_exchange_identity(ctx, msg).await;
                    if result.is_err() {
                        if let Err(err) = ctx.stop_worker(msg_addr.clone()).await {
                            warn!("cannot stop decryptor: {err} using address {msg_addr}");
                        }
                    }
                    result?
                } else {
                    // let's ignore invalid address messages errors
                    self.state = Some(State::IdentityExchange(state));
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::Initialized(_) => {
                unreachable!()
            }
        };
        self.state = Some(new_state);

        Ok(())
    }
}
