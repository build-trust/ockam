use crate::api::{DecryptionRequest, DecryptionResponse};
use crate::channel::addresses::Addresses;
use crate::channel::common::{AuthenticationConfirmation, CreateResponderChannelMessage, Role};
use crate::channel::decryptor::Decryptor;
use crate::channel::decryptor_state::{ExchangeIdentity, Initialized, KeyExchange, State};
use crate::channel::encryptor::Encryptor;
use crate::channel::encryptor_worker::EncryptorWorker;
use crate::channel::messages::IdentityChannelMessage;
use crate::{
    to_symmetric_vault, to_xx_vault, Identity, IdentityError, IdentitySecureChannelLocalInfo,
    PublicIdentity, SecureChannelRegistryEntry, SecureChannelTrustInfo, TrustPolicy,
};
use core::time::Duration;
use ockam_core::compat::vec::Vec;
use ockam_core::compat::{boxed::Box, sync::Arc};
use ockam_core::sessions::{SessionId, SessionIdLocalInfo};
use ockam_core::vault::Signature;
use ockam_core::{
    async_trait, AllowAll, AllowOnwardAddress, AllowSourceAddress, DenyAll, LocalOnwardOnly,
    LocalSourceOnly, Mailbox, Mailboxes,
};
use ockam_core::{
    route, Address, Any, Decodable, Encodable, LocalMessage, Result, Route, Routed,
    TransportMessage, Worker,
};
use ockam_core::{NewKeyExchanger, OutgoingAccessControl};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::{Context, WorkerBuilder};
use tracing::{debug, info, warn};

pub(crate) struct DecryptorWorker {
    role: Role,
    addresses: Addresses,
    // Route to the other side of the channel
    remote_route: Route,
    remote_backwards_compatibility_address: Option<Address>,
    init_payload: Option<Vec<u8>>,
    identity: Identity,
    trust_policy: Arc<dyn TrustPolicy>,
    state_key_exchange: Option<KeyExchange>,
    state_exchange_identity: Option<ExchangeIdentity>,
    state_initialized: Option<Initialized>,
    plaintext_session_id: Option<SessionId>,
}

impl DecryptorWorker {
    #[allow(clippy::too_many_arguments)]
    pub async fn create_initiator(
        ctx: &Context,
        remote_route: Route,
        identity: Identity,
        addresses: Addresses,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        plaintext_session_id: Option<SessionId>,
        timeout: Duration,
    ) -> Result<Address> {
        let mut completion_callback_ctx = ctx
            .new_detached(
                addresses.completion_callback.clone(),
                AllowSourceAddress(addresses.decryptor_callback.clone()),
                DenyAll,
            )
            .await?;

        let key_exchanger = XXNewKeyExchanger::new(to_xx_vault(identity.vault.clone()))
            .initiator()
            .await?;

        let mailboxes = Self::mailboxes(&addresses, decryptor_outgoing_access_control);

        let worker = DecryptorWorker {
            role: Role::Initiator,
            addresses: addresses.clone(),
            remote_route,
            remote_backwards_compatibility_address: None,
            init_payload: None,
            identity,
            trust_policy,
            state_key_exchange: Some(KeyExchange {
                key_exchanger: Box::new(key_exchanger),
            }),
            state_exchange_identity: None,
            state_initialized: None,
            plaintext_session_id,
        };

        WorkerBuilder::with_mailboxes(mailboxes, worker)
            .start(ctx)
            .await?;

        debug!(
            "Starting SecureChannel Initiator at remote: {}",
            &addresses.decryptor_remote
        );

        completion_callback_ctx
            .receive_timeout::<AuthenticationConfirmation>(timeout.as_secs())
            .await?;

        Ok(addresses.encryptor)
    }
}

impl DecryptorWorker {
    pub(crate) async fn create_responder(
        ctx: &Context,
        identity: Identity,
        addresses: Addresses,
        trust_policy: Arc<dyn TrustPolicy>,
        decryptor_outgoing_access_control: Arc<dyn OutgoingAccessControl>,
        plaintext_session_id: Option<SessionId>,
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

        let vault = to_xx_vault(identity.vault.clone());
        let key_exchanger = XXNewKeyExchanger::new(vault).responder().await?;

        let mailboxes = Self::mailboxes(&addresses, decryptor_outgoing_access_control);

        let worker = DecryptorWorker {
            role: Role::Responder,
            addresses: addresses.clone(),
            remote_route,
            remote_backwards_compatibility_address: Some(remote_backwards_compatibility_address),
            init_payload: Some(body.payload().to_vec()),
            identity,
            trust_policy,
            state_key_exchange: Some(KeyExchange {
                key_exchanger: Box::new(key_exchanger),
            }),
            state_exchange_identity: None,
            state_initialized: None,
            plaintext_session_id,
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
impl DecryptorWorker {
    async fn send_key_exchange_payload(
        ctx: &mut <Self as Worker>::Context,
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
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        let reply = msg.return_route();
        self.remote_route = reply;
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;

        self.handle_key_exchange(ctx, Some(&payload), false).await
    }

    async fn handle_key_exchange(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        payload: Option<&[u8]>,
        // If it's the first message
        first_run: bool,
    ) -> Result<()> {
        let state;
        if let Some(s) = self.state_key_exchange.as_mut() {
            state = s;
        } else {
            return Err(IdentityError::InvalidSecureChannelInternalState.into());
        }

        if let Some(payload) = payload {
            // Received key exchange message from remote channel, need to forward it to the local key exchange
            debug!(
                "SecureChannel received KeyExchangeRemote at {}",
                &self.addresses.decryptor_remote
            );
            let exchanger = &mut state.key_exchanger;
            let _ = exchanger.handle_response(payload).await?;
        }

        // If we'll need to generate another request
        let mut request_was_sent = false;

        // Key exchange hasn't been completed -> generate and send next request
        if !state.key_exchanger.is_complete().await? {
            request_was_sent = true;
            let payload = state.key_exchanger.generate_request(&[]).await?;

            // We should send first_responder_address only with first message from the initiator
            let custom_payload = if self.role.is_initiator() && first_run {
                Some(self.addresses.decryptor_backwards_compatibility.encode()?)
            } else {
                None
            };

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
        if !state.key_exchanger.is_complete().await? {
            return Ok(());
        }

        // Key exchange completed, proceed to Identity Exchange
        let mut state = self
            .state_key_exchange
            .take()
            .ok_or(IdentityError::InvalidSecureChannelInternalState)?;

        let keys = state.key_exchanger.finalize().await?;

        let state = ExchangeIdentity {
            encryptor: Encryptor::new(
                keys.encrypt_key().clone(),
                0,
                to_symmetric_vault(self.identity.vault.clone()),
            ),
            decryptor: Decryptor::new(
                keys.decrypt_key().clone(),
                to_symmetric_vault(self.identity.vault.clone()),
            ),
            auth_hash: *keys.h(),
            identity_sent: false,
            received_identity_id: None,
        };

        self.state_exchange_identity = Some(state);

        if !request_was_sent {
            // Key exchange was completed by processing response, no new request was required.
            // This means that it's our turn to send our Identity
            self.handle_exchange_identity(ctx, None).await?;
        } else {
            // Key exchange was completed by generating our last request.
            // This means that it's their turn to send us their Identity
            // Just wait for that message
        }

        Ok(())
    }
}

// Identity exchange
impl DecryptorWorker {
    fn state(&self) -> Result<State> {
        if self.state_key_exchange.is_some() {
            return Ok(State::KeyExchange);
        }
        if self.state_exchange_identity.is_some() {
            return Ok(State::ExchangeIdentity);
        }
        if self.state_initialized.is_some() {
            return Ok(State::Initialized);
        }

        Err(IdentityError::InvalidSecureChannelInternalState.into())
    }

    async fn handle_exchange_identity(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Option<Routed<<Self as Worker>::Message>>,
    ) -> Result<()> {
        let state;
        if let Some(s) = self.state_exchange_identity.as_mut() {
            state = s;
        } else {
            return Err(IdentityError::InvalidSecureChannelInternalState.into());
        }

        // We received an Identity
        if let Some(msg) = msg {
            if state.received_identity_id.is_some() {
                return Err(IdentityError::InvalidSecureChannelInternalState.into());
            }

            let body = Vec::<u8>::decode(msg.payload())?;
            let body = state.decryptor.decrypt(&body).await?;
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

            let their_identity =
                PublicIdentity::import(&identity, self.identity.vault.clone()).await?;
            let their_identity_id = their_identity.identifier();

            // Verify responder posses their Identity key
            let verified = their_identity
                .verify_signature(
                    &Signature::new(signature),
                    &state.auth_hash,
                    None,
                    self.identity.vault.clone(),
                )
                .await?;

            if !verified {
                return Err(IdentityError::SecureChannelVerificationFailed.into());
            }

            self.identity
                .update_known_identity(their_identity_id, &their_identity)
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

            state.received_identity_id = Some(their_identity_id.clone());
        }

        if !state.identity_sent {
            // Send our identity
            let identity = self.identity.export().await?;
            // Prove we posses our Identity key
            let signature = self
                .identity
                .create_signature(&state.auth_hash, None)
                .await?;

            let auth_msg = if state.received_identity_id.is_none() {
                IdentityChannelMessage::Request {
                    identity,
                    signature: signature.as_ref().to_vec(),
                }
            } else {
                IdentityChannelMessage::Response {
                    identity,
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

            let encrypted = state.encryptor.encrypt(&data).await?;

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

            state.identity_sent = true;
        }

        // We received and sent Identity - channel is initialized
        if let Some(their_identity_id) = state.received_identity_id.clone() {
            let old_state = self
                .state_exchange_identity
                .take()
                .ok_or(IdentityError::InvalidSecureChannelInternalState)?;
            self.state_initialized = Some(Initialized {
                decryptor: old_state.decryptor,
                their_identity_id: their_identity_id.clone(),
            });

            let next_hop = self.remote_route.next()?.clone();
            let encryptor = EncryptorWorker::new(
                self.role,
                self.addresses.clone(),
                self.remote_route.clone(),
                self.remote_backwards_compatibility_address
                    .clone()
                    .ok_or(IdentityError::InvalidSecureChannelInternalState)?,
                old_state.encryptor,
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

            WorkerBuilder::with_mailboxes(
                Mailboxes::new(main_mailbox, vec![api_mailbox]),
                encryptor,
            )
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
                self.identity.identifier().clone(),
                their_identity_id.clone(),
            );
            self.identity
                .secure_channel_registry
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
        }

        Ok(())
    }
}

// Decryption
impl DecryptorWorker {
    async fn handle_decrypt_api(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt API {}",
            self.role.str(),
            &self.addresses.decryptor_remote
        );

        let state;
        if let Some(s) = self.state_initialized.as_mut() {
            state = s;
        } else {
            return Err(IdentityError::InvalidSecureChannelInternalState.into());
        }

        let return_route = msg.return_route();

        // Decode raw payload binary
        let request = DecryptionRequest::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = state.decryptor.decrypt(&request.0).await;

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
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
    ) -> Result<()> {
        debug!(
            "SecureChannel {} received Decrypt {}",
            self.role.str(),
            &self.addresses.decryptor_remote
        );

        let state;
        if let Some(s) = self.state_initialized.as_mut() {
            state = s;
        } else {
            return Err(IdentityError::InvalidSecureChannelInternalState.into());
        }

        // Decode raw payload binary
        let payload = Vec::<u8>::decode(&msg.into_transport_message().payload)?;

        // Decrypt the binary
        let decrypted_payload = state.decryptor.decrypt(&payload).await?;

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
        let mut local_info =
            IdentitySecureChannelLocalInfo::mark(vec![], state.their_identity_id.clone())?;

        match &self.plaintext_session_id {
            Some(session_id) => {
                local_info.push(SessionIdLocalInfo::new(session_id.clone()).to_local_info()?)
            }
            None => {}
        }

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
        let init_payload = self.init_payload.take();
        // Process first received message (in case of Responder),
        // generate and send the next message
        self.handle_key_exchange(ctx, init_payload.as_deref(), true)
            .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        match self.state()? {
            State::KeyExchange => {
                if msg_addr == self.addresses.decryptor_remote {
                    self.handle_key_exchange_msg(ctx, msg).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::ExchangeIdentity => {
                if msg_addr == self.addresses.decryptor_remote {
                    self.handle_exchange_identity(ctx, Some(msg)).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::Initialized => {
                if msg_addr == self.addresses.decryptor_remote {
                    self.handle_decrypt(ctx, msg).await?;
                } else if msg_addr == self.addresses.decryptor_api {
                    self.handle_decrypt_api(ctx, msg).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
        }

        Ok(())
    }
}
