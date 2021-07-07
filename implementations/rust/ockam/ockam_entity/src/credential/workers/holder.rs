use crate::{
    check_message_origin, CredentialFragment1, CredentialProtocolMessage, CredentialSchema, Entity,
    EntityCredential, EntityError, Holder, Identity, ProfileIdentifier,
};
use async_trait::async_trait;
use ockam_core::lib::convert::TryInto;
use ockam_core::{Address, Result, Route, Routed, Worker};
use ockam_node::Context;

enum State {
    One,
    Two([u8; 96], CredentialFragment1),
    Done,
}

pub struct HolderWorker {
    entity: Entity,
    issuer_id: ProfileIdentifier,
    state: State,
    issuer_route: Route,
    schema: CredentialSchema,
    callback_address: Address,
}

impl HolderWorker {
    pub fn new(
        entity: Entity,
        issuer_id: ProfileIdentifier,
        issuer_route: Route,
        schema: CredentialSchema,
        callback_address: Address,
    ) -> Self {
        HolderWorker {
            entity,
            issuer_id,
            state: State::One,
            issuer_route,
            schema,
            callback_address,
        }
    }
}

#[async_trait]
impl Worker for HolderWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.send(
            self.issuer_route.clone(),
            CredentialProtocolMessage::IssueOfferRequest {},
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        check_message_origin(&msg, &self.issuer_id)?;

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::One => {
                if let CredentialProtocolMessage::IssueOffer(offer) = msg {
                    let issuer_contact;
                    if let Some(i) = self.entity.get_contact(&self.issuer_id)? {
                        issuer_contact = i;
                    } else {
                        return Err(EntityError::ContactNotFound.into());
                    }
                    let issuer_pubkey = issuer_contact.get_signing_public_key()?;
                    let issuer_pubkey = issuer_pubkey.as_ref().try_into().unwrap();
                    let frag = self.entity.accept_credential_offer(&offer, issuer_pubkey)?;

                    ctx.send(route, CredentialProtocolMessage::IssueRequest(frag.0))
                        .await?;

                    self.state = State::Two(issuer_pubkey, frag.1);
                }
            }
            State::Two(issuer_pubkey, frag1) => {
                if let CredentialProtocolMessage::IssueResponse(frag2) = msg {
                    let credential = self
                        .entity
                        .combine_credential_fragments(frag1.clone(), frag2)?;

                    // TODO: Save credential

                    let credential = EntityCredential {
                        credential,
                        issuer_pubkey: issuer_pubkey.clone(),
                        schema: self.schema.clone(),
                    };

                    ctx.send(self.callback_address.clone(), credential).await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
