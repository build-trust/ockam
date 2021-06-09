use crate::{
    CredentialPublicKey, ProfileRequestMessage, ProfileResponseMessage, ProfileTrait, Proof,
};
use async_trait::async_trait;
use ockam_core::{Address, Result, ResultMessage, Routed, Worker};
use ockam_node::Context;
use rand::{random, thread_rng};

/// A Worker wrapper for a Profile
pub struct ProfileWorker<P: ProfileTrait> {
    inner: P,
}

impl<P: ProfileTrait> ProfileWorker<P> {
    /// Create a new ProfileWorker
    fn new(inner: P) -> Self {
        Self { inner }
    }

    /// Create and start a ProfileWorker
    pub async fn create_with_inner(ctx: &Context, inner: P) -> Result<Address> {
        let address: Address = random();

        ctx.start_worker(address.clone(), Self::new(inner)).await?;

        Ok(address)
    }

    fn handle_request(&mut self, msg: <Self as Worker>::Message) -> Result<ProfileResponseMessage> {
        Ok(match msg {
            ProfileRequestMessage::Identifier => {
                let res = self.inner.identifier()?;
                ProfileResponseMessage::Identifier(res)
            }
            ProfileRequestMessage::ChangeEvents => {
                let res = self.inner.change_events()?;
                ProfileResponseMessage::ChangeEvents(res)
            }
            ProfileRequestMessage::UpdateNoVerification { change_event } => {
                self.inner.update_no_verification(change_event)?;
                ProfileResponseMessage::UpdateNoVerification
            }
            ProfileRequestMessage::Verify => {
                let res = self.inner.verify()?;
                ProfileResponseMessage::Verify(res)
            }
            ProfileRequestMessage::Contacts => {
                let res = self.inner.contacts()?;
                ProfileResponseMessage::Contacts(res)
            }
            ProfileRequestMessage::ToContact => {
                let res = self.inner.to_contact()?;
                ProfileResponseMessage::ToContact(res)
            }
            ProfileRequestMessage::SerializeToContact => {
                let res = self.inner.serialize_to_contact()?;
                ProfileResponseMessage::SerializeToContact(res)
            }
            ProfileRequestMessage::GetContact { id } => {
                let res = self.inner.get_contact(&id)?;
                ProfileResponseMessage::GetContact(res)
            }
            ProfileRequestMessage::VerifyContact { contact } => {
                let res = self.inner.verify_contact(&contact)?;
                ProfileResponseMessage::VerifyContact(res)
            }
            ProfileRequestMessage::VerifyAndAddContact { contact } => {
                let res = self.inner.verify_and_add_contact(contact)?;
                ProfileResponseMessage::VerifyAndAddContact(res)
            }
            ProfileRequestMessage::VerifyAndUpdateContact {
                profile_id,
                change_events,
            } => {
                let res = self
                    .inner
                    .verify_and_update_contact(&profile_id, change_events)?;
                ProfileResponseMessage::VerifyAndUpdateContact(res)
            }
            ProfileRequestMessage::GenerateAuthenticationProof { channel_state } => {
                let res = self.inner.generate_authentication_proof(&channel_state)?;
                ProfileResponseMessage::GenerateAuthenticationProof(res)
            }
            ProfileRequestMessage::VerifyAuthenticationProof {
                channel_state,
                responder_contact_id,
                proof,
            } => {
                let res = self.inner.verify_authentication_proof(
                    &channel_state,
                    &responder_contact_id,
                    &proof,
                )?;
                ProfileResponseMessage::VerifyAuthenticationProof(res)
            }
            ProfileRequestMessage::CreateKey {
                key_attributes,
                attributes,
            } => {
                self.inner.create_key(key_attributes, attributes)?;
                ProfileResponseMessage::CreateKey
            }
            ProfileRequestMessage::RotateKey {
                key_attributes,
                attributes,
            } => {
                self.inner.rotate_key(key_attributes, attributes)?;
                ProfileResponseMessage::RotateKey
            }
            ProfileRequestMessage::GetSecretKey { key_attributes } => {
                let res = self.inner.get_secret_key(&key_attributes)?;
                ProfileResponseMessage::GetSecretKey(res)
            }
            ProfileRequestMessage::GetPublicKey { key_attributes } => {
                let res = self.inner.get_public_key(&key_attributes)?;
                ProfileResponseMessage::GetPublicKey(res)
            }
            ProfileRequestMessage::GetRootSecret => {
                let res = self.inner.get_root_secret()?;
                ProfileResponseMessage::GetRootSecret(res)
            }
            // Issuer
            ProfileRequestMessage::GetSigningKey => {
                let res = self.inner.get_signing_key().unwrap();
                ProfileResponseMessage::GetSigningKey(res)
            }
            ProfileRequestMessage::GetIssuerPublicKey => {
                let res = self.inner.get_issuer_public_key().unwrap();
                ProfileResponseMessage::GetIssuerPublicKey(CredentialPublicKey(res))
            }
            ProfileRequestMessage::CreateOffer { schema } => {
                let res = self.inner.create_offer(&schema, thread_rng())?;
                ProfileResponseMessage::CreateOffer(res)
            }
            ProfileRequestMessage::CreateProofOfPossession => {
                let res = self.inner.create_proof_of_possession().unwrap();
                ProfileResponseMessage::CreateProofOfPossession(Proof(res))
            }
            ProfileRequestMessage::SignCredential { schema, attributes } => {
                let res = self.inner.sign_credential(&schema, &attributes)?;
                ProfileResponseMessage::SignCredential(res)
            }
            ProfileRequestMessage::SignCredentialRequest {
                request,
                schema,
                attributes,
                offer_id,
            } => {
                let res =
                    self.inner
                        .sign_credential_request(&request, &schema, &attributes, offer_id)?;
                ProfileResponseMessage::SignCredentialRequest(res)
            }
            // Holder
            ProfileRequestMessage::AcceptCredentialOffer { offer, public_key } => {
                let res = self
                    .inner
                    .accept_credential_offer(&offer, public_key.0, thread_rng())?;

                ProfileResponseMessage::AcceptCredentialOffer(res)
            }
            ProfileRequestMessage::CombineCredentialFragments { frag1, frag2 } => {
                let res = self.inner.combine_credential_fragments(frag1, frag2)?;
                ProfileResponseMessage::CombineCredentialFragments(res)
            }
            ProfileRequestMessage::IsValidCredential {
                credential,
                public_key,
            } => {
                let res = self
                    .inner
                    .is_valid_credential(&credential, public_key.0)
                    .unwrap();
                ProfileResponseMessage::IsValidCredential(res)
            }
            ProfileRequestMessage::PresentCredentials {
                credentials,
                manifests,
                proof_request_id,
            } => {
                let res = self.inner.present_credentials(
                    &*credentials,
                    &*manifests,
                    proof_request_id,
                    thread_rng(),
                )?;
                ProfileResponseMessage::PresentCredentials(res)
            }
            ProfileRequestMessage::CreateProofRequestId => {
                let res = self.inner.create_proof_request_id(thread_rng())?;
                ProfileResponseMessage::CreateProofRequestId(res)
            }
            ProfileRequestMessage::VerifyProofOfPossession { public_key, proof } => {
                let res = self
                    .inner
                    .verify_proof_of_possession(public_key.0, proof.0)?;
                ProfileResponseMessage::VerifyProofOfPossession(res)
            }
            ProfileRequestMessage::VerifyCredentialPresentation {
                presentations,
                manifests,
                proof_request_id,
            } => {
                let res = self.inner.verify_credential_presentations(
                    &presentations,
                    &manifests,
                    proof_request_id,
                )?;
                ProfileResponseMessage::VerifyCredentialPresentation(res)
            }
        })
    }
}

#[async_trait]
impl<P: ProfileTrait> Worker for ProfileWorker<P> {
    type Message = ProfileRequestMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let response = self.handle_request(msg.body());

        let response = ResultMessage::new(response);

        ctx.send(return_route, response).await?;

        Ok(())
    }
}
