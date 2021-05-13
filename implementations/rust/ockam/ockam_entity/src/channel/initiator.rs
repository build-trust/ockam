use crate::{
    ChannelAuthConfirm, ChannelAuthRequest, ChannelAuthResponse, Confirm, EntityError, ProfileAuth,
    ProfileContacts, ProfileImpl, ProfileVault,
};
use async_trait::async_trait;
use ockam_channel::SecureChannel;
use ockam_core::{Address, Any, Message, Result, Route, Routed, TransportMessage, Worker};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_node::Context;
use rand::random;
use tracing::{debug, info};

pub(crate) struct Initiator {
    local_secure_channel_address: Address,
    remote_profile_secure_channel_address: Option<Address>,
    self_local_address: Address,
    self_remote_address: Address,
    callback_address: Address,
}

impl Initiator {
    pub async fn create<R: Into<Route>, V: ProfileVault>(
        ctx: &Context,
        route: R,
        profile: &mut ProfileImpl<V>,
    ) -> Result<Address> {
        let vault = profile.vault();
        let new_key_exchanger = XXNewKeyExchanger::new(vault.clone());
        let route = route.into();

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

        // TODO: Add timeout
        let msg = child_ctx.receive::<ChannelAuthRequest>().await?.take();
        debug!("Received Authentication request");

        let return_route = msg.return_route();
        let msg = msg.body();

        let contact = msg.contact();
        if profile.contacts()?.contains_key(contact.identifier()) {
            // TODO: Update profile if needed
        } else {
            profile.verify_and_add_contact(contact.clone())?;
        }

        let verified = profile.verify_authentication_proof(
            &channel.auth_hash(),
            msg.contact().identifier(),
            msg.proof(),
        )?;

        if !verified {
            return Err(EntityError::SecureChannelVerificationFailed.into());
        }
        info!(
            "Verified SecureChannel from: {}",
            contact.identifier().to_string_representation()
        );

        let contact = profile.to_contact()?;
        let proof = profile.generate_authentication_proof(&channel.auth_hash())?;

        let channel_local_address: Address = random();
        let channel_remote_address: Address = random();
        let initiator = Self {
            local_secure_channel_address: channel.address(),
            remote_profile_secure_channel_address: None,
            self_local_address: channel_local_address.clone(),
            self_remote_address: channel_remote_address.clone(),
            callback_address: child_ctx.address(),
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

        let _ = child_ctx.receive::<Confirm>().await?;

        Ok(channel_local_address)
    }
}

#[async_trait]
impl Worker for Initiator {
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
            debug!("ProfileSecureChannel Initiator received Encrypt");
            let remote_profile_secure_channel_address =
                self.remote_profile_secure_channel_address.clone().unwrap(); // FIXME

            // Send to the other party
            let _ = onward_route.step()?;
            let onward_route = onward_route
                .modify()
                .prepend(remote_profile_secure_channel_address)
                .prepend(self.local_secure_channel_address.clone())
                .into();

            let return_route = return_route
                .modify()
                .append(self.self_remote_address.clone())
                .into();

            let transport_msg = TransportMessage {
                version: 1,
                onward_route,
                return_route,
                payload,
            };

            ctx.forward(transport_msg).await?;
        } else if msg_addr == self.self_remote_address {
            if self.remote_profile_secure_channel_address.is_none() {
                debug!("ProfileSecureChannel Initiator received Confirm");
                let msg = ChannelAuthConfirm::decode(&payload)?;
                self.remote_profile_secure_channel_address = Some(msg.channel_address().clone());

                ctx.send(
                    Route::new().append(self.callback_address.clone()),
                    Confirm {},
                )
                .await?;
            } else {
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
                    payload,
                };

                ctx.forward(transport_msg).await?;
            }
        } else {
            unimplemented!()
        }

        Ok(())
    }
}
