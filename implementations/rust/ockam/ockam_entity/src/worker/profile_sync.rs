use crate::{
    Contact, ContactsDb, EntityError, KeyAttributes, ProfileAuth, ProfileChangeEvent,
    ProfileChanges, ProfileContacts, ProfileEventAttributes, ProfileIdentifier, ProfileIdentity,
    ProfileRequestMessage, ProfileResponseMessage, ProfileSecrets, ProfileTrait, ProfileWorker,
};
use ockam_core::{Address, Result, ResultMessage, Route};
use ockam_node::{block_future, Context};
use ockam_vault_core::{PublicKey, Secret};
use rand::random;
use tracing::debug;

pub struct ProfileSync {
    ctx: Context,
    profile_worker_address: Address,
}

impl ProfileSync {
    pub(crate) async fn send_message(&self, m: ProfileRequestMessage) -> Result<Context> {
        let address: Address = random();
        let child_ctx = self.ctx.new_context(address).await?;
        child_ctx
            .send(Route::new().append(self.profile_worker_address.clone()), m)
            .await?;

        Ok(child_ctx)
    }

    pub(crate) async fn receive_message(ctx: &mut Context) -> Result<ProfileResponseMessage> {
        ctx.receive::<ResultMessage<ProfileResponseMessage>>()
            .await?
            .take()
            .body()
            .into()
    }
}

impl Clone for ProfileSync {
    fn clone(&self) -> Self {
        self.start_another().unwrap()
    }
}

impl ProfileSync {
    /// Start another Vault at the same address.
    pub fn start_another(&self) -> Result<Self> {
        let profile_worker_address = self.profile_worker_address.clone();

        let clone = Self::create_with_worker(&self.ctx, &profile_worker_address)?;

        Ok(clone)
    }
}

impl ProfileSync {
    /// Create and start a new Vault using Worker.
    pub fn create_with_worker(ctx: &Context, profile: &Address) -> Result<Self> {
        let address: Address = random();

        debug!("Starting ProfileSync at {}", &address);

        let ctx = block_future(
            &ctx.runtime(),
            async move { ctx.new_context(address).await },
        )?;

        Ok(Self {
            ctx,
            profile_worker_address: profile.clone(),
        })
    }

    pub async fn create<P: ProfileTrait>(ctx: &Context, profile: P) -> Result<Self> {
        let profile_address = ProfileWorker::create_with_inner(ctx, profile).await?;

        Self::create_with_worker(ctx, &profile_address)
    }
}

impl ProfileIdentity for ProfileSync {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self.send_message(ProfileRequestMessage::Identifier).await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::Identifier(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl ProfileChanges for ProfileSync {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::ChangeEvents)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::ChangeEvents(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::UpdateNoVerification { change_event })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::UpdateNoVerification = resp {
                Ok(())
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify(&mut self) -> Result<bool> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self.send_message(ProfileRequestMessage::Verify).await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::Verify(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl ProfileAuth for ProfileSync {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GenerateAuthenticationProof {
                    channel_state: channel_state.to_vec(),
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GenerateAuthenticationProof(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyAuthenticationProof {
                    channel_state: channel_state.to_vec(),
                    responder_contact_id: responder_contact_id.clone(),
                    proof: proof.to_vec(),
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyAuthenticationProof(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl ProfileContacts for ProfileSync {
    fn contacts(&self) -> Result<ContactsDb> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self.send_message(ProfileRequestMessage::Contacts).await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::Contacts(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn to_contact(&self) -> Result<Contact> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self.send_message(ProfileRequestMessage::ToContact).await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::ToContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::SerializeToContact)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::SerializeToContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetContact { id: id.clone() })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyContact {
                    contact: contact.clone(),
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyAndAddContact { contact })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyAndAddContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyAndUpdateContact {
                    profile_id: profile_id.clone(),
                    change_events,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyAndUpdateContact(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl ProfileSecrets for ProfileSync {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::CreateKey {
                    key_attributes,
                    attributes,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::CreateKey = resp {
                Ok(())
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::RotateKey {
                    key_attributes,
                    attributes,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::RotateKey = resp {
                Ok(())
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetSecretKey {
                    key_attributes: key_attributes.clone(),
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetSecretKey(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetPublicKey {
                    key_attributes: key_attributes.clone(),
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetPublicKey(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn get_root_secret(&mut self) -> Result<Secret> {
        block_future(&self.ctx.runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetRootSecret)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetRootSecret(s) = resp {
                Ok(s)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}
