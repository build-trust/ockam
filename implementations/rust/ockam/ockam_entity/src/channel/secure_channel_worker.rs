use crate::{
    ChannelAuthConfirm, ChannelAuthRequest, ChannelAuthResponse, EntityError, Identity, LocalInfo,
    ProfileIdentifier, SecureChannelTrustInfo, TrustPolicy,
};
use async_trait::async_trait;
use ockam_channel::{CreateResponderChannelMessage, KeyExchangeCompleted, SecureChannel};
use ockam_core::{
    route, Address, Any, LocalMessage, Message, Result, Route, Routed, TransportMessage, Worker,
};
use ockam_key_exchange_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Context;
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

#[derive(Serialize, Deserialize)]
struct AuthenticationConfirmation;

pub(crate) struct SecureChannelWorker {
    is_initiator: bool,
    local_secure_channel_address: Address,
    remote_profile_secure_channel_address: Option<Address>,
    self_local_address: Address,
    self_remote_address: Address,
    their_profile_id: ProfileIdentifier,
    callback_address: Option<Address>,
}

impl SecureChannelWorker {
    pub async fn create_initiator<T: TrustPolicy, P: Identity, V: XXVault>(
        ctx: &Context,
        route: Route,
        profile: &mut P,
        trust_policy: T,
        vault: V,
    ) -> Result<Address> {
        let new_key_exchanger = XXNewKeyExchanger::new(vault.clone());

        // Address used for ProfileAuth requests/responses
        let child_address: Address = random();
        let mut child_ctx = ctx.new_context(child_address).await?;

        let channel = SecureChannel::create_extended(
            ctx,
            route.clone(),
            Some(child_ctx.address()),
            &new_key_exchanger,
            vault.clone(),
        )
        .await?;

        // Wait for responder to send us his Profile and Profile Proof.
        // In case of using Noise XX this is m4 message.
        let msg = child_ctx.receive::<ChannelAuthRequest>().await?.take();
        debug!("Received Authentication request");

        let return_route = msg.return_route();
        let msg = msg.body();

        let their_contact = msg.contact();
        let their_profile_id = their_contact.identifier().clone();

        let contact_result = profile.get_contact(&their_profile_id);

        if let Some(_) = contact_result? {
            return Err(EntityError::NotImplemented.into());
        } else {
            // TODO: We're creating SecureChannel with known Profile. Need to update their Profile.
            profile.verify_and_add_contact((*their_contact).clone())?;
        }

        // Verify responder posses their Profile key
        let verified =
            profile.verify_proof(&channel.auth_hash(), &their_profile_id, msg.proof())?;

        if !verified {
            return Err(EntityError::SecureChannelVerificationFailed.into());
        }
        info!(
            "Verified SecureChannel from: {}",
            their_profile_id.to_external()
        );

        // Check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(their_profile_id.clone());
        let trusted = trust_policy.check(&trust_info)?;
        if !trusted {
            return Err(EntityError::SecureChannelTrustCheckFailed.into());
        }
        info!(
            "Checked trust policy for SecureChannel from: {}",
            their_profile_id.to_external()
        );

        // Prove we posses our Profile key
        let contact = profile.as_contact()?;
        let proof = profile.create_proof(&channel.auth_hash())?;

        // Generate 2 random fresh address for newly created SecureChannel.
        // One for local workers to encrypt their messages
        // Second for remote workers to decrypt their messages
        let channel_local_address: Address = random();
        let channel_remote_address: Address = random();
        let initiator = Self {
            is_initiator: true,
            local_secure_channel_address: channel.address(),
            remote_profile_secure_channel_address: None,
            self_local_address: channel_local_address.clone(),
            self_remote_address: channel_remote_address.clone(),
            their_profile_id: their_profile_id.clone(),
            callback_address: Some(child_ctx.address()),
        };
        debug!(
            "Starting ProfileSecureChannel Initiator at local: {}, remote: {}",
            &channel_local_address, &channel_remote_address
        );
        ctx.start_worker(
            vec![
                channel_local_address.clone(),
                channel_remote_address.clone(),
            ],
            initiator,
        )
        .await?;

        let auth_msg = ChannelAuthResponse::new(contact, proof, channel_remote_address);
        child_ctx.send(return_route, auth_msg).await?;
        debug!("Sent Authentication response");

        let _ = child_ctx.receive::<AuthenticationConfirmation>().await?;

        Ok(channel_local_address)
    }

    pub async fn create_responder<T: TrustPolicy, P: Identity>(
        ctx: &Context,
        profile: &mut P,
        trust_policy: T,
        listener_address: Address,
        msg: Routed<CreateResponderChannelMessage>,
    ) -> Result<()> {
        let mut onward_route = msg.onward_route();
        onward_route.step()?;
        onward_route.modify().prepend(listener_address);

        let return_route = msg.return_route();
        let body = msg.body();
        // This is the address of Worker on the other end, that Initiator gave us to perform further negotiations.
        let first_responder_address = body
            .completed_callback_address()
            .clone()
            .ok_or(EntityError::SecureChannelCannotBeAuthenticated)?;

        // Address used for ProfileAuth requests/responses
        let child_address: Address = random();
        let mut child_ctx = ctx.new_context(child_address).await?;
        // Change completed callback address and forward message for regular key exchange to happen
        let body =
            CreateResponderChannelMessage::new(body.payload().clone(), Some(child_ctx.address()));

        let msg = TransportMessage::v1(onward_route, return_route, body.encode()?);

        ctx.forward(LocalMessage::new(msg, Vec::new())).await?;

        // Wait for KeyExchange to happen
        let kex_msg = child_ctx
            .receive::<KeyExchangeCompleted>()
            .await?
            .take()
            .body();
        let auth_hash = kex_msg.auth_hash();

        // Prove we posses Profile key
        let proof = profile.create_proof(&auth_hash)?;
        let msg = ChannelAuthRequest::new(profile.as_contact()?, proof);
        child_ctx
            .send(
                route![kex_msg.address().clone(), first_responder_address],
                msg,
            )
            .await?;
        debug!("Sent Authentication request");

        let auth_msg = child_ctx.receive::<ChannelAuthResponse>().await?.take();
        let auth_msg = auth_msg.body();
        debug!("Received Authentication response");

        let their_contact = auth_msg.contact();
        let their_profile_id = their_contact.identifier().clone();

        let contact_result = profile.get_contact(&their_profile_id);

        if let Some(_) = contact_result? {
            // TODO: We're creating SecureChannel with known Profile. Need to update their Profile.
            return Err(EntityError::NotImplemented.into());
        } else {
            profile.verify_and_add_contact(their_contact.clone())?;
        }

        // Verify initiator posses their Profile key
        let verified =
            profile.verify_proof(&kex_msg.auth_hash(), &their_profile_id, auth_msg.proof())?;

        if !verified {
            return Err(EntityError::SecureChannelVerificationFailed.into());
        }
        info!(
            "Verified SecureChannel from: {}",
            their_profile_id.to_external()
        );

        // Check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(their_profile_id.clone());
        let trusted = trust_policy.check(&trust_info)?;
        if !trusted {
            return Err(EntityError::SecureChannelTrustCheckFailed.into());
        }
        info!(
            "Checked trust policy for SecureChannel from: {}",
            their_profile_id.to_external()
        );

        let channel_local_address: Address = random();
        let channel_remote_address: Address = random();
        let responder = Self {
            is_initiator: false,
            local_secure_channel_address: kex_msg.address().clone(),
            remote_profile_secure_channel_address: Some(auth_msg.channel_address().clone()),
            self_local_address: channel_local_address.clone(),
            self_remote_address: channel_remote_address.clone(),
            their_profile_id: their_profile_id.clone(),
            callback_address: None,
        };
        debug!(
            "Starting ProfileSecureChannel Responder at local: {}, remote: {}",
            &channel_local_address, &channel_remote_address
        );
        ctx.start_worker(
            vec![
                channel_remote_address.clone(),
                channel_local_address.clone(),
            ],
            responder,
        )
        .await?;

        child_ctx
            .send(
                route![
                    kex_msg.address().clone(),
                    auth_msg.channel_address().clone()
                ],
                ChannelAuthConfirm::new(channel_remote_address),
            )
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for SecureChannelWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();
        let msg = msg.into_transport_message();
        let payload = msg.payload;
        let mut onward_route = msg.onward_route;
        let mut return_route = msg.return_route;

        if msg_addr == self.self_local_address {
            debug!(
                "ProfileSecureChannel {} received Encrypt",
                if self.is_initiator {
                    "Initiator"
                } else {
                    "Responder"
                }
            );
            let remote_profile_secure_channel_address;
            if let Some(a) = self.remote_profile_secure_channel_address.as_ref() {
                remote_profile_secure_channel_address = a.clone();
            } else {
                return Err(EntityError::InvalidSecureChannelInternalState.into());
            }

            // Send to the other party using local regular SecureChannel
            let _ = onward_route.step()?;
            let onward_route = onward_route
                .modify()
                .prepend(remote_profile_secure_channel_address)
                .prepend(self.local_secure_channel_address.clone());

            let return_route = return_route
                .modify()
                .prepend(self.self_remote_address.clone());

            let transport_msg = TransportMessage::v1(onward_route, return_route, payload);

            ctx.forward(LocalMessage::new(transport_msg, Vec::new()))
                .await?;
        } else if msg_addr == self.self_remote_address {
            if self.is_initiator && self.remote_profile_secure_channel_address.is_none() {
                debug!("ProfileSecureChannel Initiator received Confirm");
                let msg = ChannelAuthConfirm::decode(&payload)?;
                self.remote_profile_secure_channel_address = Some(msg.channel_address().clone());

                let callback_address;
                if let Some(a) = self.callback_address.as_ref() {
                    callback_address = a.clone();
                } else {
                    return Err(EntityError::InvalidSecureChannelInternalState.into());
                }

                ctx.send(route![callback_address], AuthenticationConfirmation {})
                    .await?;
            } else {
                debug!(
                    "ProfileSecureChannel {} received Decrypt",
                    if self.is_initiator {
                        "Initiator"
                    } else {
                        "Responder"
                    }
                );

                // Ensure message came from dedicated local SecureChannel?
                let prev_hop = return_route.next()?;
                if prev_hop != &self.local_secure_channel_address {
                    return Err(EntityError::UnknownChannelMsgOrigin.into());
                }

                // Forward to local workers
                let _ = onward_route.step()?;

                let return_route = return_route
                    .modify()
                    .pop_front()
                    .pop_front()
                    .prepend(self.self_local_address.clone());

                let transport_msg = TransportMessage::v1(onward_route, return_route, payload);

                let local_info = LocalInfo::new(self.their_profile_id.clone());
                let local_info = local_info.encode()?;

                ctx.forward(LocalMessage::new(transport_msg, local_info))
                    .await?;
            }
        } else {
            return Err(EntityError::UnknownChannelMsgDestination.into());
        }

        Ok(())
    }
}
