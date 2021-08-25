use crate::{
    check_message_origin, CredentialAttribute, CredentialProtocolMessage, CredentialSchema,
    EntityError, Issuer, OfferId, Profile, ProfileIdentifier,
};
use async_trait::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{Result, Route, Routed, Worker};
use ockam_node::Context;

enum State {
    CreateOffer(Route),
    SignRequest(OfferId),
    Done,
}

pub struct IssuerWorker {
    state: State,
    profile: Profile,
    holder_id: ProfileIdentifier,
    schema: CredentialSchema,
}

impl IssuerWorker {
    pub fn new(
        profile: Profile,
        holder_id: ProfileIdentifier,
        schema: CredentialSchema,
        return_route: Route,
    ) -> Result<Self> {
        let s = Self {
            profile,
            state: State::CreateOffer(return_route),
            holder_id,
            schema,
        };

        Ok(s)
    }
}

#[async_trait]
impl Worker for IssuerWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if let State::CreateOffer(return_route) = &self.state {
            let offer = self.profile.create_offer(&self.schema)?;
            let offer_id = offer.id.clone();
            ctx.send(
                return_route.clone(),
                CredentialProtocolMessage::IssueOffer(offer),
            )
            .await?;

            self.state = State::SignRequest(offer_id);
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        check_message_origin(&msg, &self.holder_id)?;

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::CreateOffer(_) => return Err(EntityError::InvalidIssueState.into()),
            State::SignRequest(offer_id) => {
                if let CredentialProtocolMessage::IssueRequest(request, values) = msg {
                    let signing_attributes: Vec<(String, CredentialAttribute)> = self
                        .schema
                        .attributes
                        .iter()
                        .skip(1) // FIXME: SECRET_ID
                        .zip(values.iter())
                        .map(|x|
                        // FIXME: Check types?
                        (x.0.label.clone(), x.1.clone()))
                        .collect();

                    // Office signs the credentials.
                    let frag2 = self.profile.sign_credential_request(
                        &request,
                        &self.schema,
                        &(signing_attributes.clone()),
                        offer_id.clone(),
                    )?;

                    ctx.send(route, CredentialProtocolMessage::IssueResponse(frag2))
                        .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                } else {
                    return Err(EntityError::IssuerInvalidMessage.into());
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
