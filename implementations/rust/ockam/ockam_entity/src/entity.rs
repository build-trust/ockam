use crate::EntityError::IdentityApiFailed;
use crate::{
    profile::Profile, AuthenticationProof, Changes, Contact, EntityBuilder, Handle, Identity,
    IdentityRequest, IdentityResponse, Lease, MaybeContact, ProfileChangeEvent, ProfileIdentifier,
    SecureChannels, TrustPolicy, TrustPolicyImpl, TTL,
};
use ockam_core::async_trait::async_trait;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::traits::AsyncClone;
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
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

    pub async fn async_handle(&self) -> Handle {
        self.handle.async_clone().await
    }

    pub fn create(ctx: &Context, vault_address: &Address) -> Result<Entity> {
        EntityBuilder::new(ctx, vault_address)?.build()
    }

    pub async fn async_create(ctx: &Context, vault_address: &Address) -> Result<Entity> {
        let builder = EntityBuilder::async_new(ctx, vault_address).await?;
        builder.async_build().await
    }

    pub fn call(&self, req: IdentityRequest) -> Result<IdentityResponse> {
        self.handle.call(req)
    }

    pub async fn async_call(&self, req: IdentityRequest) -> Result<IdentityResponse> {
        self.handle.async_call(req).await
    }

    pub fn cast(&self, req: IdentityRequest) -> Result<()> {
        self.handle.cast(req)
    }

    pub async fn async_cast(&self, req: IdentityRequest) -> Result<()> {
        self.handle.async_cast(req).await
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

    pub async fn async_create_profile(&mut self, vault_address: &Address) -> Result<Profile> {
        if let Res::CreateProfile(id) = self
            .async_call(CreateProfile(vault_address.clone()))
            .await?
        {
            // Set current_profile_id, if it's first profile
            if self.current_profile_id.is_none() {
                self.current_profile_id = Some(id.clone());
            }
            Ok(Profile::new(id, self.handle.async_clone().await))
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

    pub async fn async_current_profile(&mut self) -> Option<Profile> {
        match &self.current_profile_id {
            None => None,
            Some(id) => Some(Profile::new(id.clone(), self.handle.async_clone().await)),
        }
    }
}

#[async_trait]
impl AsyncClone for Entity {
    async fn async_clone(&self) -> Entity {
        Entity {
            handle: self.handle.async_clone().await,
            current_profile_id: self.current_profile_id.clone(),
        }
    }
}

#[async_trait]
impl Identity for Entity {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.current_profile_id.as_ref().unwrap().clone())
    }

    async fn async_identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.current_profile_id.as_ref().unwrap().clone())
    }

    fn create_key<S: Into<String> + Send + 'static>(&mut self, label: S) -> Result<()> {
        self.cast(CreateKey(self.id(), label.into()))
    }

    async fn async_create_key<S: Into<String> + Send + 'static>(&mut self, label: S) -> Result<()> {
        self.async_cast(CreateKey(self.id(), label.into())).await
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

    async fn async_create_auth_proof<S: AsRef<[u8]> + Send + Sync>(
        &mut self,
        state_slice: S,
    ) -> Result<AuthenticationProof> {
        if let Res::CreateAuthenticationProof(proof) = self
            .async_call(CreateAuthenticationProof(
                self.id(),
                state_slice.as_ref().to_vec(),
            ))
            .await?
        {
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

    async fn async_verify_auth_proof<S: AsRef<[u8]> + Send + Sync, P: AsRef<[u8]> + Send + Sync>(
        &mut self,
        state_slice: S,
        peer_id: &ProfileIdentifier,
        proof_slice: P,
    ) -> Result<bool> {
        if let Res::VerifyAuthenticationProof(verified) = self
            .async_call(VerifyAuthenticationProof(
                self.id(),
                state_slice.as_ref().to_vec(),
                peer_id.clone(),
                proof_slice.as_ref().to_vec(),
            ))
            .await?
        {
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

    async fn async_get_changes(&self) -> Result<Changes> {
        if let Res::GetChanges(changes) = self.async_call(GetChanges(self.id())).await? {
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

    async fn async_as_contact(&mut self) -> Result<Contact> {
        let mut profile = self
            .async_current_profile()
            .await
            .expect("no current profile");
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

    async fn async_get_contact(
        &mut self,
        contact_id: &ProfileIdentifier,
    ) -> Result<Option<Contact>> {
        if let Res::GetContact(contact) = self
            .async_call(GetContact(self.id(), contact_id.clone()))
            .await?
        {
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

    async fn async_verify_contact<C: Into<Contact> + Send>(&mut self, contact: C) -> Result<bool> {
        if let Res::VerifyContact(contact) = self
            .async_call(VerifyContact(self.id(), contact.into()))
            .await?
        {
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

    async fn async_verify_and_add_contact<C: Into<Contact> + Send>(
        &mut self,
        contact: C,
    ) -> Result<bool> {
        if let Res::VerifyAndAddContact(verified_and_added) = self
            .async_call(VerifyAndAddContact(self.id(), contact.into()))
            .await?
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

#[async_trait]
impl SecureChannels for Entity {
    fn create_secure_channel_listener(
        &mut self,
        address: impl Into<Address> + Send,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let profile = self.current_profile().expect("no current profile");
        let ctx = &self.handle().ctx;
        let trust_policy_address = block_future(&self.handle.ctx.runtime(), async move {
            TrustPolicyImpl::create_worker(ctx, trust_policy).await
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
        let ctx = &self.handle().ctx;
        let trust_policy_address = block_future(&self.handle.ctx.runtime(), async move {
            TrustPolicyImpl::create_worker(ctx, trust_policy).await
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

    async fn async_create_secure_channel_listener<A>(
        &mut self,
        address: A,
        trust_policy: impl TrustPolicy,
    ) -> Result<()>
    where
        A: Into<Address> + Send,
    {
        let profile = self
            .async_current_profile()
            .await
            .expect("no current profile");
        let ctx = &self.async_handle().await.ctx;
        let trust_policy_address = TrustPolicyImpl::create_worker(ctx, trust_policy).await?;
        if let Res::CreateSecureChannelListener = self
            .async_call(CreateSecureChannelListener(
                profile
                    .async_identifier()
                    .await
                    .expect("couldn't get profile id"),
                address.into(),
                trust_policy_address,
            ))
            .await?
        {
            Ok(())
        } else {
            err()
        }
    }

    async fn async_create_secure_channel<R>(
        &mut self,
        route: R,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address>
    where
        R: Into<Route> + Send,
    {
        let profile = self
            .async_current_profile()
            .await
            .expect("no current profile");
        let ctx = &self.async_handle().await.ctx;
        let trust_policy_address = TrustPolicyImpl::create_worker(ctx, trust_policy).await?;
        if let Res::CreateSecureChannel(address) = self
            .async_call(CreateSecureChannel(
                profile
                    .async_identifier()
                    .await
                    .expect("couldn't get profile id"),
                route.into(),
                trust_policy_address,
            ))
            .await?
        {
            Ok(address)
        } else {
            err()
        }
    }
}
