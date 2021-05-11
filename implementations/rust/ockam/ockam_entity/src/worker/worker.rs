use crate::{ProfileRequestMessage, ProfileResponseMessage, ProfileTrait};
use async_trait::async_trait;
use ockam_core::{Address, Result, ResultMessage, Routed, Worker};
use ockam_node::Context;
use rand::random;

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
