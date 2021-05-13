use crate::{
    ChannelAuthConfirm, ChannelAuthRequest, ChannelAuthResponse, EntityError, ProfileAuth,
    ProfileContacts, ProfileImpl, ProfileVault,
};
use async_trait::async_trait;
use ockam_channel::{CreateResponderChannelMessage, KeyExchangeCompleted};
use ockam_core::{Address, Any, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_node::Context;
use rand::random;
use tracing::{debug, info};

pub(crate) struct Responder {
    local_secure_channel_address: Address,
    remote_profile_secure_channel_address: Address,
    self_local_address: Address,
    self_remote_address: Address,
}

impl Responder {
    pub async fn create<V: ProfileVault>(
        ctx: &Context,
        profile: &mut ProfileImpl<V>,
        listener_address: Address,
        msg: Routed<CreateResponderChannelMessage>,
    ) -> Result<()> {
        let mut onward_route = msg.onward_route();
        onward_route.step()?;
        onward_route.modify().prepend(listener_address);

        let return_route = msg.return_route();
        let body = msg.body();
        let first_responder_address = body
            .callback_address()
            .clone()
            .ok_or(EntityError::SecureChannelCannotBeAuthenticated)?;

        let child_address: Address = random();
        let mut child_ctx = ctx.new_context(child_address).await?;
        let body =
            CreateResponderChannelMessage::new(body.payload().clone(), Some(child_ctx.address()));

        let msg = TransportMessage {
            version: 1,
            onward_route,
            return_route,
            payload: body.encode()?,
        };

        ctx.forward(msg).await?;

        let kex_msg = child_ctx
            .receive::<KeyExchangeCompleted>()
            .await?
            .take()
            .body();
        let auth_hash = kex_msg.auth_hash();

        let proof = profile.generate_authentication_proof(&auth_hash)?;
        let msg = ChannelAuthRequest::new(profile.to_contact()?, proof);
        child_ctx
            .send(
                Route::new()
                    .append(kex_msg.address().clone())
                    .append(first_responder_address),
                msg,
            )
            .await?;
        debug!("Sent Authentication request");

        let auth_msg = child_ctx.receive::<ChannelAuthResponse>().await?.take();
        let auth_msg = auth_msg.body();
        debug!("Received Authentication response");

        let contact = auth_msg.contact();
        if profile.contacts()?.contains_key(contact.identifier()) {
            // TODO: Update profile if needed
        } else {
            profile.verify_and_add_contact(contact.clone())?;
        }
        let verified = profile.verify_authentication_proof(
            &kex_msg.auth_hash(),
            contact.identifier(),
            auth_msg.proof(),
        )?;

        if !verified {
            return Err(EntityError::SecureChannelVerificationFailed.into());
        }
        info!(
            "Verified SecureChannel from: {}",
            contact.identifier().to_string_representation()
        );

        let channel_local_address: Address = random();
        let channel_remote_address: Address = random();
        let responder = Self {
            local_secure_channel_address: kex_msg.address().clone(),
            remote_profile_secure_channel_address: auth_msg.channel_address().clone(),
            self_local_address: channel_local_address.clone(),
            self_remote_address: channel_remote_address.clone(),
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
                Route::new()
                    .append(kex_msg.address().clone())
                    .append(auth_msg.channel_address().clone()),
                ChannelAuthConfirm::new(channel_remote_address),
            )
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for Responder {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let msg_addr = msg.msg_addr();
        let msg = msg.into_transport_message();
        let mut onward_route = msg.onward_route;
        let mut return_route = msg.return_route;

        if msg_addr == self.self_local_address {
            debug!("ProfileSecureChannel Initiator received Encrypt");
            let remote_profile_secure_channel_address =
                self.remote_profile_secure_channel_address.clone(); // FIXME

            // Send to the other party
            let _ = onward_route.step()?;
            let onward_route = onward_route
                .modify()
                .prepend(self.local_secure_channel_address.clone())
                .prepend(remote_profile_secure_channel_address)
                .into();

            let return_route = return_route
                .modify()
                .append(self.self_remote_address.clone())
                .into();

            let transport_msg = TransportMessage {
                version: 1,
                onward_route,
                return_route,
                payload: msg.payload,
            };

            ctx.forward(transport_msg).await?;
        } else if msg_addr == self.self_remote_address {
            debug!("ProfileSecureChannel Initiator received Decrypt");
            // Forward to local workers
            let _ = onward_route.step()?;

            let return_route = return_route
                .modify()
                .append(self.self_local_address.clone())
                .into();

            let transport_msg = TransportMessage {
                version: 1,
                onward_route,
                return_route,
                payload: msg.payload,
            };

            ctx.forward(transport_msg).await?;
        } else {
            unimplemented!()
        }

        Ok(())
    }
}
