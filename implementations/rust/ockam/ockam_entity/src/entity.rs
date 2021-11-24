use crate::EntityError::IdentityApiFailed;
use crate::{
    profile::Profile, AuthenticationProof, Changes, Contact, EntityBuilder, Identity,
    IdentityRequest, IdentityResponse, Lease, MaybeContact, ProfileChangeEvent, ProfileIdentifier,
    TrustPolicy, TrustPolicyImpl, TTL,
};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, AsyncTryClone, Result, Route};
use ockam_node::{Context, Handle};
use ockam_vault::ockam_vault_core::{PublicKey, Secret};
use IdentityRequest::*;
use IdentityResponse as Res;
#[derive(AsyncTryClone)]
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

    pub async fn create(ctx: &Context, vault_address: &Address) -> Result<Entity> {
        EntityBuilder::new(ctx, vault_address).await?.build().await
    }

    pub async fn call(&self, req: IdentityRequest) -> Result<IdentityResponse> {
        self.handle.call(req).await
    }

    pub async fn cast(&self, req: IdentityRequest) -> Result<()> {
        self.handle.cast(req).await
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
    pub async fn create_profile(&mut self, vault_address: &Address) -> Result<Profile> {
        if let Res::CreateProfile(id) = self.call(CreateProfile(vault_address.clone())).await? {
            // Set current_profile_id, if it's first profile
            if self.current_profile_id.is_none() {
                self.current_profile_id = Some(id.clone());
            }
            Ok(Profile::new(id, self.handle.async_try_clone().await?))
        } else {
            err()
        }
    }

    pub async fn remove_profile<I: Into<ProfileIdentifier>>(
        &mut self,
        profile_id: I,
    ) -> Result<()> {
        self.cast(RemoveProfile(profile_id.into())).await
    }

    pub async fn current_profile(&self) -> Result<Option<Profile>> {
        match &self.current_profile_id {
            None => Ok(None),
            Some(id) => Ok(Some(Profile::new(
                id.clone(),
                self.handle.async_try_clone().await?,
            ))),
        }
    }
}

#[async_trait]
impl Identity for Entity {
    async fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.current_profile_id.as_ref().unwrap().clone())
    }

    async fn create_key(&mut self, label: String) -> Result<()> {
        self.cast(CreateKey(self.id(), label)).await
    }

    async fn add_key(&mut self, label: String, secret: &Secret) -> Result<()> {
        if let Res::AddKey = self.call(AddKey(self.id(), label, secret.clone())).await? {
            Ok(())
        } else {
            err()
        }
    }

    async fn rotate_root_secret_key(&mut self) -> Result<()> {
        self.cast(RotateKey(self.id())).await
    }

    async fn get_root_secret_key(&self) -> Result<Secret> {
        if let Res::GetProfileSecretKey(secret) = self.call(GetProfileSecretKey(self.id())).await? {
            Ok(secret)
        } else {
            err()
        }
    }

    async fn get_secret_key(&self, label: String) -> Result<Secret> {
        if let Res::GetSecretKey(secret) = self.call(GetSecretKey(self.id(), label)).await? {
            Ok(secret)
        } else {
            err()
        }
    }

    async fn get_root_public_key(&self) -> Result<PublicKey> {
        if let Res::GetProfilePublicKey(public_key) =
            self.call(GetProfilePublicKey(self.id())).await?
        {
            Ok(public_key)
        } else {
            err()
        }
    }

    async fn get_public_key(&self, label: String) -> Result<PublicKey> {
        if let Res::GetPublicKey(public_key) = self.call(GetPublicKey(self.id(), label)).await? {
            Ok(public_key)
        } else {
            err()
        }
    }

    async fn create_auth_proof(&mut self, state_slice: &[u8]) -> Result<AuthenticationProof> {
        if let Res::CreateAuthenticationProof(proof) = self
            .call(CreateAuthenticationProof(
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

    async fn verify_auth_proof(
        &mut self,
        state_slice: &[u8],
        peer_id: &ProfileIdentifier,
        proof_slice: &[u8],
    ) -> Result<bool> {
        if let Res::VerifyAuthenticationProof(verified) = self
            .call(VerifyAuthenticationProof(
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

    async fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        self.cast(AddChange(self.id(), change_event)).await
    }

    async fn get_changes(&self) -> Result<Changes> {
        if let Res::GetChanges(changes) = self.call(GetChanges(self.id())).await? {
            Ok(changes)
        } else {
            err()
        }
    }

    async fn verify_changes(&mut self) -> Result<bool> {
        if let Res::VerifyChanges(verified) = self.call(VerifyChanges(self.id())).await? {
            Ok(verified)
        } else {
            err()
        }
    }

    async fn get_contacts(&self) -> Result<Vec<Contact>> {
        if let Res::Contacts(contact) = self.call(GetContacts(self.id())).await? {
            Ok(contact)
        } else {
            err()
        }
    }

    async fn as_contact(&mut self) -> Result<Contact> {
        let mut profile = self
            .current_profile()
            .await
            .unwrap()
            .expect("no current profile");
        let contact = profile.as_contact().await?;
        Ok(contact)
    }

    async fn get_contact(&mut self, contact_id: &ProfileIdentifier) -> Result<Option<Contact>> {
        if let Res::GetContact(contact) =
            self.call(GetContact(self.id(), contact_id.clone())).await?
        {
            match contact {
                MaybeContact::None => Ok(None),
                MaybeContact::Contact(contact) => Ok(Some(contact)),
            }
        } else {
            err()
        }
    }

    async fn verify_contact(&mut self, contact: Contact) -> Result<bool> {
        if let Res::VerifyContact(contact) = self.call(VerifyContact(self.id(), contact)).await? {
            Ok(contact)
        } else {
            err()
        }
    }

    async fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        if let Res::VerifyAndAddContact(verified_and_added) =
            self.call(VerifyAndAddContact(self.id(), contact)).await?
        {
            Ok(verified_and_added)
        } else {
            err()
        }
    }

    async fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        changes: &[ProfileChangeEvent],
    ) -> Result<bool> {
        if let Res::VerifyAndUpdateContact(verified_and_updated) = self
            .call(VerifyAndUpdateContact(
                self.id(),
                profile_id.clone(),
                changes.as_ref().to_vec(),
            ))
            .await?
        {
            Ok(verified_and_updated)
        } else {
            err()
        }
    }

    async fn get_lease(
        &self,
        lease_manager_route: &Route,
        org_id: String,
        bucket: String,
        ttl: TTL,
    ) -> Result<Lease> {
        if let Res::Lease(lease) = self
            .call(GetLease(
                lease_manager_route.clone(),
                self.id(),
                org_id.to_string(),
                bucket.to_string(),
                ttl,
            ))
            .await?
        {
            Ok(lease)
        } else {
            err()
        }
    }

    async fn revoke_lease(&mut self, lease_manager_route: &Route, lease: Lease) -> Result<()> {
        self.cast(RevokeLease(lease_manager_route.clone(), self.id(), lease))
            .await
    }
}

impl Entity {
    pub async fn create_secure_channel_listener(
        &mut self,
        address: impl Into<Address>,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let profile = self
            .current_profile()
            .await
            .unwrap()
            .expect("no current profile");
        let ctx = self.handle.ctx();
        let trust_policy_address = TrustPolicyImpl::create_worker(ctx, trust_policy).await?;
        if let Res::CreateSecureChannelListener = self
            .call(CreateSecureChannelListener(
                profile.identifier().await.expect("couldn't get profile id"),
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

    pub async fn create_secure_channel(
        &mut self,
        route: impl Into<Route>,
        trust_policy: impl TrustPolicy,
    ) -> Result<Address> {
        let profile = self
            .current_profile()
            .await
            .unwrap()
            .expect("no current profile");
        let ctx = self.handle.ctx();
        let trust_policy_address = TrustPolicyImpl::create_worker(ctx, trust_policy).await?;
        if let Res::CreateSecureChannel(address) = self
            .call(CreateSecureChannel(
                profile.identifier().await.expect("couldn't get profile id"),
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
