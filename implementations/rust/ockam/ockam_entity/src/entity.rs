use crate::EntityError::IdentityApiFailed;
use crate::{
    profile::Profile, traits::Verifier, AuthenticationProof, BbsCredential, Changes, Contact,
    Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialProof, CredentialProtocol, CredentialPublicKey,
    CredentialRequest, CredentialRequestFragment, CredentialSchema, EntityBuilder,
    EntityCredential, Handle, Holder, HolderWorker, Identity, IdentityRequest, IdentityResponse,
    Issuer, Lease, ListenerWorker, MaybeContact, OfferId, PresentationFinishedMessage,
    PresentationManifest, PresenterWorker, ProfileChangeEvent, ProfileIdentifier, ProofRequestId,
    SecureChannels, SigningPublicKey, TrustPolicy, TrustPolicyImpl, VerifierWorker, TTL,
};
use core::convert::TryInto;
use ockam_core::{Address, Result, Route};
use ockam_node::{block_future, Context};
use ockam_vault::ockam_vault_core::{PublicKey, Secret};
use signature_bls::SecretKey;
use IdentityRequest::*;
use IdentityResponse as Res;

#[derive(Clone)]
pub struct Entity {
    handle: Handle,
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
}

impl Issuer for Entity {
    fn get_signing_key(&mut self) -> Result<SecretKey> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::GetSigningKey(signing_key) = profile.call(GetSigningKey(
            profile.identifier().expect("couldn't get profile id"),
        ))? {
            Ok(signing_key)
        } else {
            err()
        }
    }

    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        // FIXME: Why clone?
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::GetIssuerPublicKey(issuer_public_key) = profile.call(GetIssuerPublicKey(
            profile.identifier().expect("couldn't get profile id"),
        ))? {
            Ok(issuer_public_key.0)
        } else {
            err()
        }
    }

    fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CreateOffer(offer) = profile.call(CreateOffer(
            profile.identifier().expect("couldn't get profile id"),
            schema.clone(),
        ))? {
            Ok(offer)
        } else {
            err()
        }
    }

    fn create_proof_of_possession(&self) -> Result<CredentialProof> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CreateProofOfPossession(proof) = profile.call(CreateProofOfPossession(
            profile.identifier().expect("couldn't get profile id"),
        ))? {
            Ok(proof)
        } else {
            err()
        }
    }

    fn sign_credential<A: AsRef<[CredentialAttribute]>>(
        &self,
        schema: &CredentialSchema,
        attributes: A,
    ) -> Result<BbsCredential> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::SignCredential(credential) = profile.call(SignCredential(
            profile.identifier().expect("couldn't get profile id"),
            schema.clone(),
            attributes.as_ref().to_vec(),
        ))? {
            Ok(credential)
        } else {
            err()
        }
    }

    fn sign_credential_request<A: AsRef<[(String, CredentialAttribute)]>>(
        &self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: A,
        offer_id: OfferId,
    ) -> Result<CredentialFragment2> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::SignCredentialRequest(frag) = profile.call(SignCredentialRequest(
            profile.identifier().expect("couldn't get profile id"),
            request.clone(),
            schema.clone(),
            attributes.as_ref().to_vec(),
            offer_id,
        ))? {
            Ok(frag)
        } else {
            err()
        }
    }
}

impl Holder for Entity {
    fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_public_key: SigningPublicKey,
    ) -> Result<CredentialRequestFragment> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::AcceptCredentialOffer(request_fragment) =
            profile.call(AcceptCredentialOffer(
                profile.identifier().expect("couldn't get profile id"),
                offer.clone(),
                CredentialPublicKey(issuer_public_key),
            ))?
        {
            Ok(request_fragment)
        } else {
            err()
        }
    }

    fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CombineCredentialFragments(credential) =
            profile.call(CombineCredentialFragments(
                profile.identifier().expect("couldn't get profile id"),
                credential_fragment1,
                credential_fragment2,
            ))?
        {
            Ok(credential)
        } else {
            err()
        }
    }

    fn is_valid_credential(
        &self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::IsValidCredential(valid) = profile.call(IsValidCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
            CredentialPublicKey(verifier_key),
        ))? {
            Ok(valid)
        } else {
            err()
        }
    }

    fn create_credential_presentation(
        &self,
        credential: &BbsCredential,
        presentation_manifests: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<CredentialPresentation> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::PresentCredential(presentation) = profile.call(PresentCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
            presentation_manifests.clone(),
            proof_request_id,
        ))? {
            Ok(presentation)
        } else {
            err()
        }
    }

    fn add_credential(&mut self, credential: EntityCredential) -> Result<()> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::AddCredential = profile.call(AddCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential,
        ))? {
            Ok(())
        } else {
            err()
        }
    }

    fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::GetCredential(c) = profile.call(GetCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
        ))? {
            Ok(c)
        } else {
            err()
        }
    }
}

impl Verifier for Entity {
    fn create_proof_request_id(&self) -> Result<ProofRequestId> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CreateProofRequestId(request_id) = profile.call(CreateProofRequestId(
            profile.identifier().expect("couldn't get profile id"),
        ))? {
            Ok(request_id)
        } else {
            err()
        }
    }

    fn verify_proof_of_possession(
        &self,
        signing_public_key: CredentialPublicKey,
        proof: CredentialProof,
    ) -> Result<bool> {
        let profile = self.clone().current_profile().expect("no current profile");

        if let Res::VerifyProofOfPossession(verified) = profile.call(VerifyProofOfPossession(
            profile.identifier().expect("couldn't get profile id"),
            signing_public_key,
            proof,
        ))? {
            Ok(verified)
        } else {
            err()
        }
    }

    fn verify_credential_presentation(
        &self,
        presentation: &CredentialPresentation,
        presentation_manifest: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::VerifyCredentialPresentation(verified) =
            profile.call(VerifyCredentialPresentation(
                profile.identifier().expect("couldn't get profile id"),
                presentation.clone(),
                presentation_manifest.clone(),
                proof_request_id,
            ))?
        {
            Ok(verified)
        } else {
            err()
        }
    }
}

impl CredentialProtocol for Entity {
    fn create_credential_issuance_listener(
        &mut self,
        address: impl Into<Address> + Send,
        schema: CredentialSchema,
        trust_policy: impl TrustPolicy,
    ) -> Result<()> {
        let profile = self.clone().current_profile().expect("no current profile");
        block_future(&self.handle.ctx.runtime(), async move {
            let trust_policy =
                TrustPolicyImpl::create_using_impl(&self.handle.ctx, trust_policy).await?;

            let address = address.into();
            let worker = ListenerWorker::new(profile, schema, trust_policy);
            self.handle.ctx.start_worker(address, worker).await?;

            Ok(())
        })
    }

    fn acquire_credential(
        &mut self,
        issuer_route: Route,
        issuer_id: &ProfileIdentifier,
        schema: CredentialSchema,
        values: Vec<CredentialAttribute>,
    ) -> Result<Credential> {
        let profile = self.clone().current_profile().expect("no current profile");
        block_future(&self.handle.ctx.runtime(), async move {
            let mut ctx = self.handle.ctx.new_context(Address::random(0)).await?;

            let worker = HolderWorker::new(
                profile.clone(),
                issuer_id.clone(),
                issuer_route,
                schema,
                values,
                ctx.address(),
            );
            ctx.start_worker(Address::random(0), worker).await?;

            let credential = ctx
                .receive_timeout::<Credential>(120 /* FIXME */)
                .await?
                .take()
                .body();

            Ok(credential)
        })
    }

    fn present_credential(
        &mut self,
        verifier_route: Route,
        credential: Credential,
        reveal_attributes: Vec<String>,
    ) -> Result<()> {
        let profile = self.clone().current_profile().expect("no current profile");
        block_future(&self.handle.ctx.runtime(), async move {
            let credential = self.get_credential(&credential)?;

            let mut ctx = self.handle.ctx.new_context(Address::random(0)).await?;
            let worker = PresenterWorker::new(
                profile.clone(),
                verifier_route,
                credential,
                reveal_attributes,
                ctx.address(),
            );
            ctx.start_worker(Address::random(0), worker).await?;

            let _ = ctx
                .receive_timeout::<PresentationFinishedMessage>(120 /* FIXME */)
                .await?
                .take()
                .body();

            Ok(())
        })
    }

    fn verify_credential(
        &mut self,
        address: impl Into<Address> + Send,
        issuer_id: &ProfileIdentifier,
        schema: CredentialSchema,
        attributes_values: Vec<CredentialAttribute>,
    ) -> Result<bool> {
        let mut profile = self.clone().current_profile().expect("no current profile");
        block_future(&self.handle.ctx.runtime(), async move {
            let issuer = profile.get_contact(issuer_id)?.unwrap();
            let pubkey = issuer.get_signing_public_key()?;

            let mut ctx = self.handle.ctx.new_context(Address::random(0)).await?;
            let worker = VerifierWorker::new(
                profile.clone(),
                pubkey.as_ref().try_into().unwrap(), // FIXME
                schema,
                attributes_values,
                ctx.address(),
            );

            ctx.start_worker(address.into(), worker).await?;

            let res = ctx
                .receive_timeout::<bool>(120 /* FIXME */)
                .await?
                .take()
                .body();

            Ok(res)
        })
    }
}
