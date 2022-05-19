use crate::{
    EncryptorWorker, IdentityChannelMessage, IdentityError, IdentityIdentifier,
    IdentitySecureChannelLocalInfo, IdentityTrait, SecureChannelTrustInfo, TrustPolicy,
};
use core::future::Future;
use core::pin::Pin;
use core::time::Duration;
use ockam_channel::{
    CreateResponderChannelMessage, KeyExchangeCompleted, SecureChannel, SecureChannelDecryptor,
    SecureChannelInfo,
};
use ockam_core::async_trait;
use ockam_core::compat::rand::random;
use ockam_core::compat::{boxed::Box, sync::Arc, vec::Vec};
use ockam_core::{
    route, Address, Any, Decodable, Encodable, LocalMessage, Message, Result, Route, Routed,
    TransportMessage, Worker,
};
use ockam_key_exchange_core::NewKeyExchanger;
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[derive(Serialize, Deserialize, Message)]
pub(crate) struct AuthenticationConfirmation(pub Address);

trait StartSecureChannelFuture: Future<Output = Result<SecureChannelInfo>> + Send + 'static {}

impl<T> StartSecureChannelFuture for T where
    T: Future<Output = Result<SecureChannelInfo>> + Send + 'static
{
}

struct InitiatorStartChannel<I: IdentityTrait> {
    channel_future: Pin<Box<dyn StartSecureChannelFuture>>, // TODO: Replace with generic
    callback_address: Address,
    identity: I,
    trust_policy: Arc<dyn TrustPolicy>,
}

struct ResponderWaitForKex<I: IdentityTrait> {
    first_responder_address: Address,
    identity: I,
    trust_policy: Arc<dyn TrustPolicy>,
}

struct InitiatorSendIdentity<I: IdentityTrait> {
    channel: SecureChannelInfo,
    callback_address: Address,
    identity: I,
    trust_policy: Arc<dyn TrustPolicy>,
}

struct ResponderWaitForIdentity<I: IdentityTrait> {
    auth_hash: [u8; 32],
    local_secure_channel_address: Address,
    identity: I,
    trust_policy: Arc<dyn TrustPolicy>,
}

#[derive(Clone)]
struct Initialized {
    local_secure_channel_address: Address,
    their_identity_id: IdentityIdentifier,
    encryptor_address: Address,
}

enum State<I: IdentityTrait> {
    InitiatorStartChannel(InitiatorStartChannel<I>),
    ResponderWaitForKex(ResponderWaitForKex<I>),
    InitiatorSendIdentity(InitiatorSendIdentity<I>),
    ResponderWaitForIdentity(ResponderWaitForIdentity<I>),
    Initialized(Initialized),
}

pub(crate) struct DecryptorWorker<I: IdentityTrait> {
    is_initiator: bool,
    self_address: Address,
    kex_callback_address: Option<Address>,
    state: Option<State<I>>,
}

impl<I: IdentityTrait> DecryptorWorker<I> {
    pub async fn create_initiator(
        ctx: &Context,
        route: Route,
        identity: I,
        trust_policy: Arc<dyn TrustPolicy>,
        vault: impl XXVault,
        timeout: Duration,
    ) -> Result<Address> {
        let child_address = Address::random_local();
        let mut child_ctx = ctx.new_detached(child_address.clone()).await?;

        let self_address: Address = random();

        let initiator = XXNewKeyExchanger::new(vault.async_try_clone().await?)
            .initiator()
            .await?;
        // Create regular secure channel and set self address as first responder
        let custom_payload = self_address.encode()?;
        let temp_ctx = ctx.new_detached(Address::random_local()).await?;
        let channel_future = Box::pin(async move {
            SecureChannel::create_extended(&temp_ctx, route, Some(custom_payload), initiator, vault)
                .await
        });

        let state = State::InitiatorStartChannel(InitiatorStartChannel {
            channel_future,
            callback_address: child_address,
            identity,
            trust_policy,
        });

        let worker = DecryptorWorker {
            is_initiator: true,
            self_address: self_address.clone(),
            kex_callback_address: None,
            state: Some(state),
        };

        ctx.start_worker(self_address.clone(), worker).await?;

        debug!(
            "Starting IdentitySecureChannel Initiator at remote: {}",
            &self_address
        );

        let encryptor_address = child_ctx
            .receive_timeout::<AuthenticationConfirmation>(timeout.as_secs())
            .await?
            .take()
            .body()
            .0;

        Ok(encryptor_address)
    }

    pub(crate) async fn create_responder(
        ctx: &Context,
        identity: I,
        trust_policy: Arc<dyn TrustPolicy>,
        vault: impl XXVault,
        msg: Routed<CreateResponderChannelMessage>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let mut onward_route = msg.onward_route();
        let body = msg.body();
        // This is the address of Worker on the other end, that Initiator gave us to perform further negotiations.
        let custom_payload = body
            .custom_payload()
            .as_ref()
            .ok_or(IdentityError::SecureChannelCannotBeAuthenticated)?;
        let first_responder_address = Address::decode(custom_payload)?;

        let self_address: Address = random();

        let state = State::ResponderWaitForKex(ResponderWaitForKex {
            first_responder_address,
            identity,
            trust_policy,
        });

        let kex_callback_address = Address::random_local();
        let worker = DecryptorWorker {
            is_initiator: false,
            self_address: self_address.clone(),
            kex_callback_address: Some(kex_callback_address.clone()),
            state: Some(state),
        };

        ctx.start_worker(
            vec![self_address.clone(), kex_callback_address.clone()],
            worker,
        )
        .await?;

        debug!(
            "Starting IdentitySecureChannel Responder at remote: {}",
            &self_address
        );

        let regular_responder_address = Address::random_local();

        let responder = XXNewKeyExchanger::new(vault.async_try_clone().await?)
            .responder()
            .await?;

        let vault = vault.async_try_clone().await?;
        let regular_decryptor =
            SecureChannelDecryptor::new_responder(responder, Some(kex_callback_address), vault)
                .await?;

        ctx.start_worker(vec![regular_responder_address.clone()], regular_decryptor)
            .await?;

        onward_route.step()?;
        onward_route.modify().prepend(regular_responder_address);

        let msg = TransportMessage::v1(onward_route, return_route, body.payload().encode()?);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        Ok(())
    }

    async fn handle_kex_done(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
        state: ResponderWaitForKex<I>,
    ) -> Result<()> {
        let kex_msg = KeyExchangeCompleted::decode(msg.payload())?;

        // Prove we posses Identity key
        let proof = state
            .identity
            .create_auth_proof(&kex_msg.auth_hash())
            .await?;
        let msg = IdentityChannelMessage::Request {
            contact: state.identity.as_contact().await?,
            proof,
        };
        ctx.send_from_address(
            route![kex_msg.address().clone(), state.first_responder_address],
            msg,
            self.self_address.clone(),
        )
        .await?;
        debug!("Sent Authentication request");

        self.state = Some(State::ResponderWaitForIdentity(ResponderWaitForIdentity {
            auth_hash: kex_msg.auth_hash(),
            local_secure_channel_address: kex_msg.address().clone(),
            identity: state.identity,
            trust_policy: Arc::clone(&state.trust_policy),
        }));

        Ok(())
    }

    async fn handle_send_identity(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
        state: InitiatorSendIdentity<I>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        // Ensure message came from dedicated SecureChannel
        if return_route.next()? != &state.channel.address() {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        let body = IdentityChannelMessage::decode(msg.payload())?;

        // Wait for responder to send us his Identity and Identity Proof.
        // In case of using Noise XX this is m4 message.
        if let IdentityChannelMessage::Request { contact, proof } = body {
            debug!("Received Authentication request");

            let their_contact = contact;
            let their_identity_id = their_contact.identifier().clone();

            let contact_result = state.identity.get_contact(&their_identity_id).await?;

            if contact_result.is_some() {
                // TODO: We're creating SecureChannel with known Identity. Need to update their Identity.
            } else {
                state.identity.verify_and_add_contact(their_contact).await?;
            }

            // Verify responder posses their Identity key
            let verified = state
                .identity
                .verify_auth_proof(&state.channel.auth_hash(), &their_identity_id, &proof)
                .await?;

            if !verified {
                return Err(IdentityError::SecureChannelVerificationFailed.into());
            }
            info!(
                "Initiator verified SecureChannel from: {}",
                their_identity_id
            );

            // Check our TrustPolicy
            let trust_info = SecureChannelTrustInfo::new(their_identity_id.clone());
            let trusted = state.trust_policy.check(&trust_info).await?;
            if !trusted {
                return Err(IdentityError::SecureChannelTrustCheckFailed.into());
            }
            info!(
                "Initiator checked trust policy for SecureChannel from: {}",
                &their_identity_id
            );

            // Prove we posses our Identity key
            let contact = state.identity.as_contact().await?;
            let proof = state
                .identity
                .create_auth_proof(&state.channel.auth_hash())
                .await?;

            let auth_msg = IdentityChannelMessage::Response { contact, proof };

            let remote_identity_secure_channel_address = return_route.recipient();

            ctx.send_from_address(return_route, auth_msg, self.self_address.clone())
                .await?;
            debug!("Sent Authentication response");

            let encryptor_address = Address::random_local();

            self.state = Some(State::Initialized(Initialized {
                local_secure_channel_address: state.channel.address(),
                their_identity_id,
                encryptor_address: encryptor_address.clone(),
            }));

            let encryptor = EncryptorWorker::new(
                self.is_initiator,
                self.self_address.clone(),
                remote_identity_secure_channel_address,
                state.channel.address(),
            );

            ctx.start_worker(encryptor_address.clone(), encryptor)
                .await?;

            info!(
                "Initialized IdentitySecureChannel Initiator at local: {}, remote: {}",
                &encryptor_address, &self.self_address
            );

            ctx.send(
                state.callback_address,
                AuthenticationConfirmation(encryptor_address),
            )
            .await?;

            Ok(())
        } else {
            Err(IdentityError::InvalidSecureChannelInternalState.into())
        }
    }

    async fn handle_receive_identity(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
        state: ResponderWaitForIdentity<I>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        // Ensure message came from dedicated SecureChannel
        if return_route.next()? != &state.local_secure_channel_address {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        let body = IdentityChannelMessage::decode(msg.payload())?;

        // Wait for responder to send us his Identity and Identity Proof.
        // In case of using Noise XX this is m4 message.
        if let IdentityChannelMessage::Response { contact, proof } = body {
            debug!("Received Authentication response");

            let their_contact = contact;
            let their_identity_id = their_contact.identifier().clone();

            let contact_result = state.identity.get_contact(&their_identity_id).await?;

            if contact_result.is_some() {
                // TODO: We're creating SecureChannel with known Identity. Need to update their Identity.
            } else {
                state
                    .identity
                    .verify_and_add_contact(their_contact.clone())
                    .await?;
            }

            // Verify initiator posses their Identity key
            let verified = state
                .identity
                .verify_auth_proof(&state.auth_hash, &their_identity_id, &proof)
                .await?;

            if !verified {
                return Err(IdentityError::SecureChannelVerificationFailed.into());
            }

            info!(
                "Responder verified SecureChannel from: {}",
                &their_identity_id
            );

            // Check our TrustPolicy
            let trust_info = SecureChannelTrustInfo::new(their_identity_id.clone());
            let trusted = state.trust_policy.check(&trust_info).await?;
            if !trusted {
                return Err(IdentityError::SecureChannelTrustCheckFailed.into());
            }
            info!(
                "Responder checked trust policy for SecureChannel from: {}",
                &their_identity_id
            );

            let remote_identity_secure_channel_address = return_route.recipient();

            let encryptor_address = Address::random_local();

            self.state = Some(State::Initialized(Initialized {
                local_secure_channel_address: state.local_secure_channel_address.clone(),
                their_identity_id,
                encryptor_address: encryptor_address.clone(),
            }));

            let encryptor = EncryptorWorker::new(
                self.is_initiator,
                self.self_address.clone(),
                remote_identity_secure_channel_address,
                state.local_secure_channel_address,
            );

            ctx.start_worker(encryptor_address.clone(), encryptor)
                .await?;

            info!(
                "Initialized IdentitySecureChannel Responder at local: {}, remote: {}",
                &encryptor_address, &self.self_address
            );

            Ok(())
        } else {
            Err(IdentityError::InvalidSecureChannelInternalState.into())
        }
    }

    // FIXME: Avoid situation where we take state but don't put it back because of an error
    fn take_state(&mut self) -> Result<State<I>> {
        if let Some(s) = self.state.take() {
            Ok(s)
        } else {
            Err(IdentityError::InvalidSecureChannelInternalState.into())
        }
    }

    async fn handle_decrypt(
        &mut self,
        ctx: &mut <Self as Worker>::Context,
        msg: Routed<<Self as Worker>::Message>,
        state: Initialized,
    ) -> Result<()> {
        debug!(
            "IdentitySecureChannel {} received Decrypt",
            if self.is_initiator {
                "Initiator"
            } else {
                "Responder"
            }
        );

        self.state = Some(State::Initialized(state.clone()));

        let mut onward_route = msg.onward_route();
        let mut return_route = msg.return_route();

        // Ensure message came from dedicated SecureChannel
        if return_route.next()? != &state.local_secure_channel_address {
            return Err(IdentityError::UnknownChannelMsgDestination.into());
        }

        let local_msg = msg.into_local_message();
        let mut local_info = local_msg.local_info().to_vec();
        let payload = local_msg.into_transport_message().payload;

        // Forward to local workers
        let _ = onward_route.step()?;

        let return_route = return_route
            .modify()
            .pop_front()
            .pop_front()
            .prepend(state.encryptor_address.clone());

        let transport_msg = TransportMessage::v1(onward_route, return_route, payload);

        local_info.push(
            IdentitySecureChannelLocalInfo::new(state.their_identity_id.clone()).to_local_info()?,
        );

        let msg = LocalMessage::new(transport_msg, local_info);

        match ctx.forward(msg).await {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    "{} forwarding decrypted message from {}",
                    err, state.encryptor_address
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl<I: IdentityTrait> Worker for DecryptorWorker<I> {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        if self.is_initiator {
            match self.take_state()? {
                State::InitiatorStartChannel(s) => {
                    let channel = s.channel_future.await?;

                    self.state = Some(State::InitiatorSendIdentity(InitiatorSendIdentity {
                        channel,
                        callback_address: s.callback_address,
                        identity: s.identity,
                        trust_policy: s.trust_policy,
                    }));
                }
                _ => return Err(IdentityError::InvalidSecureChannelInternalState.into()),
            }
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();

        match self.take_state()? {
            State::InitiatorStartChannel(_) => {
                return Err(IdentityError::InvalidSecureChannelInternalState.into())
            }
            State::ResponderWaitForKex(s) => {
                let kex_callback_address = self
                    .kex_callback_address
                    .take()
                    .ok_or(IdentityError::UnknownChannelMsgDestination)?;
                if msg_addr == kex_callback_address {
                    self.handle_kex_done(ctx, msg, s).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::InitiatorSendIdentity(s) => {
                if msg_addr == self.self_address {
                    self.handle_send_identity(ctx, msg, s).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::ResponderWaitForIdentity(s) => {
                if msg_addr == self.self_address {
                    self.handle_receive_identity(ctx, msg, s).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
            State::Initialized(s) => {
                if msg_addr == self.self_address {
                    self.handle_decrypt(ctx, msg, s).await?;
                } else {
                    return Err(IdentityError::UnknownChannelMsgDestination.into());
                }
            }
        }

        Ok(())
    }
}
