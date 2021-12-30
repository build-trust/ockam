use crate::{
    Credential, CredentialAcquisitionResultMessage, CredentialAttribute, CredentialFragment1,
    CredentialProtocolMessage, CredentialSchema, EntityCredential, EntityError,
    EntitySecureChannelLocalInfo, Holder, Identity, Profile, ProfileIdentifier, SigningPublicKey,
};
use core::convert::TryInto;
use ockam_core::async_trait;
use ockam_core::compat::rand::random;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{Address, Result, Route, Routed, Worker};
use ockam_node::Context;

enum State {
    AcceptOffer,
    CombineFragments(SigningPublicKey, CredentialFragment1),
    Done,
}

pub struct HolderWorker {
    state: State,
    profile: Profile,
    issuer_id: ProfileIdentifier,
    issuer_route: Route,
    schema: CredentialSchema,
    values: Vec<CredentialAttribute>,
    callback_address: Address,
}

impl HolderWorker {
    pub fn new(
        profile: Profile,
        issuer_id: ProfileIdentifier,
        issuer_route: Route,
        schema: CredentialSchema,
        values: Vec<CredentialAttribute>,
        callback_address: Address,
    ) -> Self {
        Self {
            profile,
            issuer_id,
            state: State::AcceptOffer,
            issuer_route,
            schema,
            values,
            callback_address,
        }
    }
}

#[async_trait]
impl Worker for HolderWorker {
    type Context = Context;
    type Message = CredentialProtocolMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        // TODO: Set access control

        ctx.send(
            self.issuer_route.clone(),
            CredentialProtocolMessage::IssueOfferRequest(self.schema.id.clone()),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let local_info = EntitySecureChannelLocalInfo::find_info(msg.local_message())?;
        if self.issuer_id.ne(local_info.their_profile_id()) {
            return Err(EntityError::HolderInvalidMessage.into());
        }

        let route = msg.return_route();
        let msg = msg.body();

        match &self.state {
            State::AcceptOffer => {
                if let CredentialProtocolMessage::IssueOffer(offer) = msg {
                    let issuer_contact;
                    if let Some(i) = self.profile.get_contact(&self.issuer_id).await? {
                        issuer_contact = i;
                    } else {
                        return Err(EntityError::ContactNotFound.into());
                    }
                    let issuer_pubkey = issuer_contact.get_signing_public_key()?;
                    let issuer_pubkey = issuer_pubkey.as_ref().try_into().unwrap();
                    let frag = self
                        .profile
                        .accept_credential_offer(&offer, issuer_pubkey)
                        .await?;

                    ctx.send(
                        route,
                        CredentialProtocolMessage::IssueRequest(frag.0, self.values.clone()),
                    )
                    .await?;

                    self.state = State::CombineFragments(issuer_pubkey, frag.1);
                } else {
                    return Err(EntityError::HolderInvalidMessage.into());
                }
            }
            State::CombineFragments(issuer_pubkey, frag1) => {
                if let CredentialProtocolMessage::IssueResponse(frag2) = msg {
                    let bbs_credential = self
                        .profile
                        .combine_credential_fragments(frag1.clone(), frag2)
                        .await?;

                    let credential =
                        Credential::new(random(), self.issuer_id.clone(), self.schema.id.clone());

                    let entity_credential = EntityCredential::new(
                        credential.clone(),
                        bbs_credential,
                        issuer_pubkey.clone(),
                        self.schema.clone(),
                    );

                    self.profile.add_credential(entity_credential).await?;

                    ctx.send(
                        self.callback_address.clone(),
                        CredentialAcquisitionResultMessage { credential },
                    )
                    .await?;

                    self.state = State::Done;

                    ctx.stop_worker(ctx.address()).await?;
                } else {
                    return Err(EntityError::HolderInvalidMessage.into());
                }
            }
            State::Done => {}
        }

        Ok(())
    }
}
