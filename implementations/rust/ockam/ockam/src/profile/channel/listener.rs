use crate::{async_worker, Contact, OckamError, Profile};
use ockam_channel::{CreateResponderChannelMessage, KeyExchangeCompleted, SecureChannel};
use ockam_core::{Address, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_vault_sync_core::VaultSync;
use rand::random;
use serde::{Deserialize, Serialize};

pub struct ProfileChannelListener {
    profile: Profile, // TODO: Avoid copying profile
    vault: VaultSync,
    listener_address: Option<Address>,
}

impl ProfileChannelListener {
    pub fn new(profile: Profile, vault: VaultSync) -> Self {
        ProfileChannelListener {
            profile,
            vault,
            listener_address: None,
        }
    }
}

#[async_worker]
impl Worker for ProfileChannelListener {
    type Message = CreateResponderChannelMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let listener_address: Address = random();
        SecureChannel::create_listener_with_vault_sync(
            ctx,
            listener_address.clone(),
            self.vault.start_another()?,
        )
        .await?;

        self.listener_address = Some(listener_address);

        Ok(())
    }

    fn shutdown(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
        // TODO: ctx.stop_worker(self.listener_address.take().unwrap()).await
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let mut onward_route = msg.onward_route();
        onward_route.step()?;
        onward_route
            .modify()
            .prepend(self.listener_address.clone().unwrap());

        let return_route = msg.return_route();
        let body = msg.body();
        let body = CreateResponderChannelMessage::new(body.payload().clone(), Some(ctx.address()));

        let msg = TransportMessage {
            version: 1,
            onward_route,
            return_route,
            payload: body.encode()?,
        };

        ctx.forward(msg).await?;

        let m = ctx.receive::<KeyExchangeCompleted>().await?.take().body();

        let auth_msg = ctx.receive::<ChannelAuthMessage>().await?.take();
        let return_route = auth_msg.return_route();
        let auth_msg = auth_msg.body();

        if auth_msg.auth_hash() != m.auth_hash() {
            return Err(OckamError::SecureChannelVerificationFailed.into());
        }

        let contact = auth_msg.contact();
        if self.profile.contacts().contains_key(contact.identifier()) {
            // TODO: Update profile if needed
        } else {
            self.profile.verify_and_add_contact(contact.clone())?;
        }
        let verified = self.profile.verify_authentication_proof(
            &m.auth_hash(),
            contact.identifier(),
            auth_msg.proof(),
        )?;

        if !verified {
            return Err(OckamError::SecureChannelVerificationFailed.into());
        }

        let proof = self.profile.generate_authentication_proof(&m.auth_hash())?;
        let msg = ChannelAuthMessage::new(m.auth_hash(), self.profile.to_contact(), proof);
        ctx.send(return_route, msg).await?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChannelAuthMessage {
    auth_hash: [u8; 32],
    contact: Contact,
    proof: Vec<u8>,
}

impl ChannelAuthMessage {
    pub fn auth_hash(&self) -> [u8; 32] {
        self.auth_hash
    }
    pub fn contact(&self) -> &Contact {
        &self.contact
    }
    pub fn proof(&self) -> &Vec<u8> {
        &self.proof
    }
}

impl ChannelAuthMessage {
    pub fn new(auth_hash: [u8; 32], contact: Contact, proof: Vec<u8>) -> Self {
        ChannelAuthMessage {
            auth_hash,
            contact,
            proof,
        }
    }
}
