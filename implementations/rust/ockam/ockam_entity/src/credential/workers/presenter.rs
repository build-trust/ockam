use crate::{
    CredentialProtocolMessage, EntityCredential, EntityError, Holder, PresentationFinishedMessage,
    PresentationManifest, Profile,
};
use async_trait::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{Address, Result, Route, Routed, Worker};
use ockam_node::Context;

enum State {
    CreatePresentation,
    Done,
}

pub struct PresenterWorker {
    state: State,
    profile: Profile,
    verifier_route: Route,
    credential: EntityCredential,
    reveal_attributes: Vec<String>,
    callback_address: Address,
}

impl PresenterWorker {
    pub fn new(
        profile: Profile,
        verifier_route: Route,
        credential: EntityCredential,
        reveal_attributes: Vec<String>,
        callback_address: Address,
    ) -> Self {
        Self {
            state: State::CreatePresentation,
            profile,
            verifier_route,
            credential,
            reveal_attributes,
            callback_address,
        }
    }
}

#[async_trait]
impl Worker for PresenterWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.send(
            self.verifier_route.clone(),
            CredentialProtocolMessage::PresentationOffer,
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // FIXME: Should we check that? check_message_origin(&msg, &self.verifier_id)?;

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::CreatePresentation => {
                if let CredentialProtocolMessage::PresentationRequest(request_id) = msg {
                    let schema = self.credential.schema();
                    let revealed = self
                        .reveal_attributes
                        .iter()
                        .map(
                            |x| {
                                schema
                                    .attributes
                                    .iter()
                                    .position(|y| x == &y.label)
                                    .unwrap()
                            }, // FIXME
                        )
                        .collect();

                    let manifest = PresentationManifest {
                        credential_schema: self.credential.schema().clone(),
                        public_key: self.credential.issuer_pubkey(),
                        revealed,
                    };

                    let presentation = self.profile.create_credential_presentation(
                        self.credential.bbs_credential(),
                        &manifest,
                        request_id,
                    )?;

                    ctx.send(
                        route,
                        CredentialProtocolMessage::PresentationResponse(presentation),
                    )
                    .await?;

                    ctx.send(self.callback_address.clone(), PresentationFinishedMessage)
                        .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                } else {
                    return Err(EntityError::PresenterInvalidMessage.into());
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
