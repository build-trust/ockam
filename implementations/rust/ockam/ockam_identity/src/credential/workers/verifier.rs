use crate::credential::Verifier;
use crate::{
    CredentialAttribute, CredentialProtocolMessage, CredentialSchema,
    CredentialVerificationResultMessage, Identity, IdentityError, IdentityIdentifier,
    IdentitySecureChannelLocalInfo, PresentationManifest, ProofRequestId, SigningPublicKey,
};
use ockam_core::async_trait;
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
    identity: Identity,
    presenter_id: Option<IdentityIdentifier>,
    pubkey: SigningPublicKey,
    schema: CredentialSchema,
    attributes_values: Vec<CredentialAttribute>,
    callback_address: Address,
}

impl VerifierWorker {
    pub fn new(
        identity: Identity,
        pubkey: SigningPublicKey,
        schema: CredentialSchema,
        attributes_values: Vec<CredentialAttribute>,
        callback_address: Address,
    ) -> Self {
        Self {
            state: State::CreateRequestId,
            identity,
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
        let local_info = IdentitySecureChannelLocalInfo::find_info(msg.local_message())?;
        if let Some(presenter_id) = &self.presenter_id {
            if presenter_id.ne(local_info.their_identity_id()) {
                return Err(IdentityError::VerifierInvalidMessage.into());
            }
        } else {
            self.presenter_id = Some(local_info.their_identity_id().to_owned());
        }

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::CreateRequestId => {
                if let CredentialProtocolMessage::PresentationOffer = msg {
                    let id = self.identity.create_proof_request_id().await?;
                    ctx.send(route, CredentialProtocolMessage::PresentationRequest(id))
                        .await?;

                    self.state = State::VerifyPresentation(id);
                } else {
                    return Err(IdentityError::VerifierInvalidMessage.into());
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

                    let is_valid = self
                        .identity
                        .verify_credential_presentation(&presentation, &manifest, id.clone())
                        .await?;

                    // TODO: Add some mechanism to identify participant as an owner of the valid credential
                    ctx.send(
                        self.callback_address.clone(),
                        CredentialVerificationResultMessage { is_valid },
                    )
                    .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                } else {
                    return Err(IdentityError::VerifierInvalidMessage.into());
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
