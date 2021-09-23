use crate::EntityError::IdentityApiFailed;
use crate::{
    profile::Profile, AuthenticationProof, Changes, Contact, EntityBuilder, Identity,
    IdentityRequest, IdentityResponse, Lease, MaybeContact, ProfileChangeEvent, ProfileIdentifier,
    SecureChannels, TrustPolicy, TrustPolicyImpl, TTL,
};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context, Handle};
use ockam_vault::ockam_vault_core::{PublicKey, Secret};
use IdentityRequest::*;
use IdentityResponse as Res;

#[derive(Clone)]
pub struct Entity {
    pub(crate) handle: Handle,
    current_profile_id: Option<ProfileIdentifier>,
}

impl Entity {
    pub(crate) fn new(handle: Handle, profile_id: Option<ProfileIdentifier>) -> Self {
        Entity {
            handle,
            current_profile_id: profile_id,
        }
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn create(ctx: &Context, vault_address: &Address) -> Result<Entity> {
        EntityBuilder::new(ctx, vault_address)?.build()
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
    pub fn create_profile(&mut self, vault_address: &Address) -> Result<Profile> {
        if let Res::CreateProfile(id) = self.call(CreateProfile(vault_address.clone()))? {
            // Set current_profile_id, if it's first profile
            if self.current_profile_id.is_none() {
                self.current_profile_id = Some(id.clone());
            }
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

    fn rotate_profile_key(&mut self) -> Result<()> {
        self.cast(RotateKey(self.id()))
    }

    fn get_profile_secret_key(&self) -> Result<Secret> {
        if let Res::GetProfileSecretKey(secret) = self.call(GetProfileSecretKey(self.id()))? {
            Ok(secret)
        } else {
            err()
        }
    }

    fn get_secret_key<S: Into<String>>(&self, label: S) -> Result<Secret> {
        if let Res::GetSecretKey(secret) = self.call(GetSecretKey(self.id(), label.into()))? {
            Ok(secret)
        } else {
            err()
        }
    }

    fn get_profile_public_key(&self) -> Result<PublicKey> {
        if let Res::GetProfilePublicKey(public_key) = self.call(GetProfilePublicKey(self.id()))? {
            Ok(public_key)
        } else {
            err()
        }
    }

    fn get_public_key<S: Into<String>>(&self, label: S) -> Result<PublicKey> {
        if let Res::GetPublicKey(public_key) = self.call(GetPublicKey(self.id(), label.into()))? {
            Ok(public_key)
        } else {
            err()
        }
    }

    fn create_auth_proof<S: AsRef<[u8]>>(&mut self, state_slice: S) -> Result<AuthenticationProof> {
        if let Res::CreateAuthenticationProof(proof) = self.call(CreateAuthenticationProof(
            self.id(),
            state_slice.as_ref().to_vec(),
        ))? {
            Ok(proof)
        } else {
            err()
        }
    }

    fn verify_auth_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
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

    fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: impl ToString,
        bucket: impl ToString,
        ttl: TTL,
    ) -> Result<Lease> {
        if let Res::Lease(lease) = self.call(GetLease(
            lease_manager_route.clone(),
            self.id(),
            org_id.to_string(),
            bucket.to_string(),
            ttl,
        ))? {
            Ok(lease)
        } else {
            err()
        }
    }

    fn revoke_lease(&mut self, lease_manager_route: &Route, lease: Lease) -> Result<()> {
        self.cast(RevokeLease(lease_manager_route.clone(), self.id(), lease))
    }
}

impl SecureChannels for Entity {
    fn create_secure_channel_listener(
        &mut self,
        address: impl Into<Address> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let profile = self.current_profile().expect("no current profile");
        let handle = &self.handle();
        let trust_policy_address = block_future(&self.handle.ctx().runtime(), async move {
            TrustPolicyImpl::create_worker(handle.ctx(), trust_policy).await
        })?;
        if let Res::CreateSecureChannelListener = self.call(CreateSecureChannelListener(
            profile.identifier().expect("couldn't get profile id"),
            address.into(),
            trust_policy_address,
        ))? {
            Ok(())
        } else {
            err()
        }
    }

    fn create_secure_channel(
        &mut self,
        route: impl Into<Route> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address> {
        let profile = self.current_profile().expect("no current profile");
        let handle = &self.handle();
        let trust_policy_address = block_future(&self.handle.ctx().runtime(), async move {
            TrustPolicyImpl::create_worker(handle.ctx(), trust_policy).await
        })?;
        if let Res::CreateSecureChannel(address) = self.call(CreateSecureChannel(
            profile.identifier().expect("couldn't get profile id"),
            route.into(),
            trust_policy_address,
        ))? {
            Ok(address)
        } else {
            err()
        }
    }
}
