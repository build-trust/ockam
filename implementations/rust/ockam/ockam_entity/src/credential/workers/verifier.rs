use crate::traits::Verifier;
use crate::{
    check_message_origin, get_secure_channel_participant_id, CredentialAttribute,
    CredentialProtocolMessage, CredentialSchema, EntityError, PresentationManifest, Profile,
    ProfileIdentifier, ProofRequestId, SigningPublicKey,
};
use async_trait::async_trait;
use ockam_core::compat::{boxed::Box, string::String, vec::Vec};
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;

enum State {
    CreateRequestId,
    VerifyPresentation(ProofRequestId),
    Done,
}

pub struct VerifierWorker {
    state: State,
    profile: Profile,
    presenter_id: Option<ProfileIdentifier>,
    pubkey: SigningPublicKey,
    schema: CredentialSchema,
    attributes_values: Vec<CredentialAttribute>,
    callback_address: Address,
}

impl VerifierWorker {
    pub fn new(
        profile: Profile,
        pubkey: SigningPublicKey,
        schema: CredentialSchema,
        attributes_values: Vec<CredentialAttribute>,
        callback_address: Address,
    ) -> Self {
        Self {
            state: State::CreateRequestId,
            profile,
            presenter_id: None,
            pubkey,
            schema,
            attributes_values,
            callback_address,
        }
    }
}

#[async_trait]
impl Worker for VerifierWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        if let Some(presenter_id) = &self.presenter_id {
            check_message_origin(&msg, presenter_id)?;
        } else {
            self.presenter_id = Some(get_secure_channel_participant_id(&msg)?);
        }

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::CreateRequestId => {
                if let CredentialProtocolMessage::PresentationOffer = msg {
                    let id = self.profile.create_proof_request_id()?;
                    ctx.send(route, CredentialProtocolMessage::PresentationRequest(id))
                        .await?;

                    self.state = State::VerifyPresentation(id);
                } else {
                    return Err(EntityError::VerifierInvalidMessage.into());
                }
            }
            State::VerifyPresentation(id) => {
                if let CredentialProtocolMessage::PresentationResponse(presentation) = msg {
                    let schema = &self.schema;

                    // TODO: Are attributes_values guaranteed to match values in credential after the check? Or should we perform additional checks?
                    let attributes: Vec<(String, CredentialAttribute)> = self
                        .schema
                        .attributes
                        .iter()
                        .skip(1) // FIXME: SECRET_ID
                        .zip(self.attributes_values.iter())
                        .map(
                            |x| (x.0.label.clone(), x.1.clone()), // TODO: Support different order in schema and attribute_values?
                        )
                        .collect();

                    // TODO: Partial reveal is not supported
                    let revealed = attributes.iter().enumerate().map(|(i, _)| i + 1).collect();
                    let manifest = PresentationManifest {
                        credential_schema: schema.clone(),
                        public_key: self.pubkey,
                        revealed,
                    };

                    let credential_is_valid = self.profile.verify_credential_presentation(
                        &presentation,
                        &manifest,
                        id.clone(),
                    )?;

                    // TODO: Add some mechanism to identify participant as an owner of the valid credential
                    ctx.send(self.callback_address.clone(), credential_is_valid)
                        .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                } else {
                    return Err(EntityError::VerifierInvalidMessage.into());
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
