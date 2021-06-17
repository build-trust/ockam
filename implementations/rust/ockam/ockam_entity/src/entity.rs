use crate::{
    profile::Profile, worker::EntityWorker, Changes, Contact, EntityError::IdentityApiFailed,
    Handle, Identity, IdentityRequest, IdentityResponse, MaybeContact, ProfileChangeEvent,
    ProfileIdentifier, Proof, SecureChannels,
};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use ockam_vault::ockam_vault_core::{PublicKey, Secret};
use IdentityRequest::*;
use IdentityResponse as Res;

pub struct Entity {
    handle: Handle,
    current_profile_id: Option<ProfileIdentifier>,
}

impl Entity {
    pub fn new(handle: Handle, profile_id: ProfileIdentifier) -> Self {
        Entity {
            handle,
            current_profile_id: Some(profile_id),
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn create(node_ctx: &Context) -> Result<Entity> {
        block_future(&node_ctx.runtime(), async move {
            let ctx = node_ctx.new_context(Address::random(0)).await?;
            let address = Address::random(0);
            ctx.start_worker(&address, EntityWorker::default()).await?;

            let mut entity = Entity {
                handle: Handle::new(ctx, address),
                current_profile_id: None,
            };

            let default_profile = entity.create_profile()?;
            entity.current_profile_id = Some(default_profile.identifier()?);
            Ok(entity)
        })
    }

    pub fn call(&self, req: IdentityRequest) -> Result<IdentityResponse> {
        self.handle.call(req)
    }

    pub fn cast(&self, req: IdentityRequest) -> Result<()> {
        self.handle.cast(req)
    }
}

impl Entity {
    pub fn id(&self) -> ProfileIdentifier {
        self.current_profile_id.as_ref().unwrap().clone()
    }
}

fn err<T>() -> Result<T> {
    Err(IdentityApiFailed.into())
}

impl Entity {
    pub fn create_profile(&mut self) -> Result<Profile> {
        if let Res::CreateProfile(id) = self.call(CreateProfile)? {
            Ok(Profile::new(id, self.handle.clone()))
        } else {
            err()
        }
    }

    pub fn remove_profile<I: Into<ProfileIdentifier>>(&mut self, profile_id: I) -> Result<()> {
        self.cast(RemoveProfile(profile_id.into()))
    }

    pub fn current_profile(&mut self) -> Option<Profile> {
        match &self.current_profile_id {
            None => None,
            Some(id) => Some(Profile::new(id.clone(), self.handle.clone())),
        }
    }
}

impl Identity for Entity {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.current_profile_id.as_ref().unwrap().clone())
    }

    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()> {
        self.cast(CreateKey(self.id(), label.into()))
    }

    fn rotate_key(&mut self) -> Result<()> {
        self.cast(RotateKey(self.id()))
    }

    fn get_secret_key(&self) -> Result<Secret> {
        if let Res::GetSecretKey(secret) = self.call(GetSecretKey(self.id()))? {
            Ok(secret)
        } else {
            err()
        }
    }

    fn get_public_key(&self) -> Result<PublicKey> {
        if let Res::GetPublicKey(public_key) = self.call(GetPublicKey(self.id()))? {
            Ok(public_key)
        } else {
            err()
        }
    }

    fn create_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<Proof> {
        if let Res::CreateAuthenticationProof(proof) = self.call(CreateAuthenticationProof(
            self.id(),
            state_slice.as_ref().to_vec(),
        ))? {
            Ok(proof)
        } else {
            err()
        }
    }

    fn verify_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        state_slice: S,
        peer_id: &ProfileIdentifier,
        proof_slice: P,
    ) -> Result<bool> {
        if let Res::VerifyAuthenticationProof(verified) = self.call(VerifyAuthenticationProof(
            self.id(),
            state_slice.as_ref().to_vec(),
            peer_id.clone(),
            proof_slice.as_ref().to_vec(),
        ))? {
            Ok(verified)
        } else {
            err()
        }
    }

    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        self.cast(AddChange(self.id(), change_event))
    }

    fn get_changes(&self) -> Result<Changes> {
        if let Res::GetChanges(changes) = self.call(GetChanges(self.id()))? {
            Ok(changes)
        } else {
            err()
        }
    }

    fn verify_changes(&mut self) -> Result<bool> {
        if let Res::VerifyChanges(verified) = self.call(VerifyChanges(self.id()))? {
            Ok(verified)
        } else {
            err()
        }
    }

    fn get_contacts(&self) -> Result<Vec<Contact>> {
        if let Res::Contacts(contact) = self.call(GetContacts(self.id()))? {
            Ok(contact)
        } else {
            err()
        }
    }

    fn as_contact(&mut self) -> Result<Contact> {
        let mut profile = self.current_profile().expect("no current profile");
        let contact = profile.as_contact()?;
        Ok(contact)
    }

    fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>> {
        if let Res::GetContact(contact) = self.call(GetContact(self.id(), contact_id.clone()))? {
            match contact {
                MaybeContact::None => Ok(None),
                MaybeContact::Contact(contact) => Ok(Some(contact)),
            }
        } else {
            err()
        }
    }

    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        if let Res::VerifyContact(contact) = self.call(VerifyContact(self.id(), contact.into()))? {
            Ok(contact)
        } else {
            err()
        }
    }

    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        if let Res::VerifyAndAddContact(verified_and_added) =
            self.call(VerifyAndAddContact(self.id(), contact.into()))?
        {
            Ok(verified_and_added)
        } else {
            err()
        }
    }

    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        profile_id: &ProfileIdentifier,
        changes: C,
    ) -> Result<bool> {
        if let Res::VerifyAndUpdateContact(verified_and_updated) = self.call(
            VerifyAndUpdateContact(self.id(), profile_id.clone(), changes.as_ref().to_vec()),
        )? {
            Ok(verified_and_updated)
        } else {
            err()
        }
    }
}

impl SecureChannels for Entity {
    fn create_secure_channel_listener<A: Into<Address>>(&mut self, address: A) -> Result<()> {
        let profile = self.current_profile().expect("no current profile");
        self.cast(CreateSecureChannelListener(
            profile.identifier().expect("couldn't get profile id"),
            address.into(),
        ))
    }

    fn create_secure_channel<R: Into<Route> + Send>(&mut self, route: R) -> Result<Address> {
        let profile = self.current_profile().expect("no current profile");
        if let Res::CreateSecureChannel(address) = self.call(CreateSecureChannel(
            profile.identifier().expect("couldn't get profile id"),
            route.into(),
        ))? {
            Ok(address)
        } else {
            err()
        }
    }
}
