use crate::{
    check_message_origin, CredentialAttribute, CredentialProtocolMessage, CredentialSchema, Entity,
    Issuer, ProfileIdentifier,
};
use async_trait::async_trait;
use ockam_core::{Result, Routed, Worker};
use ockam_node::Context;

enum State {
    Offer,
    Issue([u8; 32]),
    Done,
}

pub struct IssuerWorker {
    entity: Entity,
    state: State,
    holder_id: ProfileIdentifier,
    schema: CredentialSchema,
}

impl IssuerWorker {
    pub fn new(entity: Entity, holder_id: ProfileIdentifier, schema: CredentialSchema) -> Self {
        IssuerWorker {
            entity,
            state: State::Offer,
            holder_id,
            schema,
        }
    }
}

#[async_trait]
impl Worker for IssuerWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        check_message_origin(&msg, &self.holder_id)?;

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::Offer => {
                if let CredentialProtocolMessage::IssueOfferRequest = msg {
                    // The Issuer (Office) creates an Credential Request Offer (ability to open the door)
                    let offer = self.entity.create_offer(&self.schema)?;
                    let offer_id = offer.id.clone();
                    ctx.send(route, CredentialProtocolMessage::IssueOffer(offer))
                        .await?;

                    self.state = State::Issue(offer_id);
                }
            }
            State::Issue(offer_id) => {
                if let CredentialProtocolMessage::IssueRequest(request) = msg {
                    // Ask the Issuer to sign the Credential Request. A successful request results in a second fragment.
                    // FIXME
                    let signing_attributes = [
                        (
                            // TODO: ProfileIdentifier
                            "door_id".into(),
                            // TODO: Replace with Verifier ProfileIdentifier?
                            CredentialAttribute::String("f4a8-90ff-742d-11ae".into()),
                        ),
                        ("can_open_door".into(), CredentialAttribute::Numeric(1)),
                    ];

                    // Office signs the credentials.
                    let frag2 = self.entity.sign_credential_request(
                        &request,
                        &self.schema,
                        &(signing_attributes.clone()),
                        offer_id.clone(),
                    )?;

                    ctx.send(route, CredentialProtocolMessage::IssueResponse(frag2))
                        .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
