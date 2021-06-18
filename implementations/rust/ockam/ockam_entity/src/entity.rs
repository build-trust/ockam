use crate::credential::CredentialVerifier;
use crate::protocol::messages::{
    IssueOffer, IssueOfferRequest, IssueRequest, IssueResponse, PresentationOffer,
    PresentationRequest, PresentationResponse,
};
use crate::EntityError::{InvalidInternalState, InvalidParameter, ProfileNotFound};
use crate::{
    check_message_origin, Contact, ContactsDb, Credential, CredentialAttribute,
    CredentialFragment1, CredentialFragment2, CredentialHolder, CredentialIssuer, CredentialOffer,
    CredentialPresentation, CredentialProtocol, CredentialRequest, CredentialSchema,
    EntityCredential, KeyAttributes, OfferIdBytes, PresentationManifest, Profile, ProfileAdd,
    ProfileAuth, ProfileChangeEvent, ProfileChanges, ProfileContacts, ProfileEventAttributes,
    ProfileIdentifier, ProfileIdentity, ProfileRemove, ProfileRetrieve, ProfileSecrets,
    ProfileSync, ProofBytes, ProofRequestId, PublicKeyBytes, SecureChannelTrait, TrustPolicy,
};
use async_trait::async_trait;
use ockam_core::hashbrown::HashMap;
use ockam_core::lib::convert::TryInto;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use ockam_vault_core::{PublicKey, Secret};
use ockam_vault_sync_core::Vault;
use rand::{thread_rng, CryptoRng, RngCore};
use signature_bls::SecretKey;

pub struct Entity {
    ctx: Context,
    vault: Address,
    default_profile_identifier: ProfileIdentifier,
    profiles: HashMap<ProfileIdentifier, ProfileSync>,
    secure_channels: HashMap<Address, Route>,
}

impl Entity {
    pub async fn create(node_ctx: &Context) -> Result<Entity> {
        let ctx = node_ctx.new_context(Address::random(0)).await?;
        let vault = Vault::create(&ctx)?;
        let default_profile = Profile::create(&ctx, &vault).await?;
        let default_profile_identifier = default_profile.identifier()?;

        let mut profiles = HashMap::new();
        profiles.insert(default_profile_identifier.clone(), default_profile);

        Ok(Entity {
            ctx,
            vault,
            default_profile_identifier,
            profiles,
            secure_channels: Default::default(),
        })
    }

    fn default_profile(&self) -> Option<&ProfileSync> {
        self.profile(&self.default_profile_identifier)
    }

    fn default_profile_mut(&mut self) -> Option<&mut ProfileSync> {
        let id = self.default_profile_identifier.clone();
        self.profile_mut(&id)
    }

    pub async fn secure_channel_listen_on_address<A: Into<Address>, T: TrustPolicy>(
        &mut self,
        address: A,
        trust_policy: T,
    ) -> Result<()> {
        let profile = self.profiles.get_mut(&self.default_profile_identifier);
        if profile.is_some() {
            let profile = profile.unwrap();
            profile
                .create_secure_channel_listener(
                    &self.ctx,
                    address.into(),
                    trust_policy,
                    &self.vault,
                )
                .await
        } else {
            Err(ProfileNotFound.into())
        }
    }

    pub async fn create_secure_channel_listener<T: TrustPolicy>(
        &mut self,
        secure_channel_address: &str,
        trust_policy: T,
    ) -> Result<()> {
        self.secure_channel_listen_on_address(secure_channel_address, trust_policy)
            .await
    }

    pub async fn create_secure_channel<R: Into<Route>, T: TrustPolicy>(
        &mut self,
        route: R,
        trust_policy: T,
    ) -> Result<Address> {
        let route = route.into();

        let profile = self.profiles.get_mut(&self.default_profile_identifier);
        if profile.is_some() {
            let profile = profile.unwrap();
            let channel = profile
                .create_secure_channel(&self.ctx, route.clone(), trust_policy, &self.vault)
                .await?;
            self.secure_channels.insert(channel.clone(), route);
            Ok(channel)
        } else {
            Err(InvalidInternalState.into())
        }
    }

    pub fn list_secure_channels(&self) -> Result<Vec<(&Address, &Route)>> {
        Ok(self.secure_channels.iter().collect())
    }
}

impl ProfileAdd for Entity {
    fn add_profile(&mut self, profile: ProfileSync) -> Result<()> {
        if let Ok(id) = profile.identifier() {
            if self.profiles.insert(id, profile).is_some() {
                return Ok(());
            }
        }
        Err(InvalidInternalState.into())
    }
}

impl ProfileRetrieve for Entity {
    fn profile(&self, profile_identifier: &ProfileIdentifier) -> Option<&ProfileSync> {
        self.profiles.get(profile_identifier)
    }

    fn profile_mut(&mut self, profile_identifier: &ProfileIdentifier) -> Option<&mut ProfileSync> {
        self.profiles.get_mut(profile_identifier)
    }
}

impl ProfileRemove for Entity {
    fn remove_profile(&mut self, profile_id: &ProfileIdentifier) -> Result<()> {
        if &self.default_profile_identifier == profile_id {
            return Err(InvalidParameter.into());
        }
        if self.profiles.remove(&profile_id).is_some() {
            Ok(())
        } else {
            Err(InvalidInternalState.into())
        }
    }
}

impl ProfileIdentity for Entity {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        if let Some(profile) = self.default_profile() {
            Ok(profile.identifier()?)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl ProfileChanges for Entity {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        if let Some(profile) = self.default_profile() {
            profile.change_events()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        if let Some(profile) = self.default_profile_mut() {
            profile.update_no_verification(change_event)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify(&mut self) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify()
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl ProfileSecrets for Entity {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        if let Some(profile) = self.default_profile_mut() {
            profile.create_key(key_attributes, attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        if let Some(profile) = self.default_profile_mut() {
            profile.rotate_key(key_attributes, attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        if let Some(profile) = self.default_profile_mut() {
            profile.get_secret_key(key_attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        if let Some(profile) = self.default_profile() {
            profile.get_public_key(key_attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_root_secret(&mut self) -> Result<Secret> {
        if let Some(profile) = self.default_profile_mut() {
            profile.get_root_secret()
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl ProfileContacts for Entity {
    fn contacts(&self) -> Result<ContactsDb> {
        if let Some(profile) = self.default_profile() {
            profile.contacts()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn to_contact(&self) -> Result<Contact> {
        if let Some(profile) = self.default_profile() {
            profile.to_contact()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        if let Some(profile) = self.default_profile() {
            profile.serialize_to_contact()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        if let Some(profile) = self.default_profile() {
            profile.get_contact(id)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_contact(contact)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_and_add_contact(contact)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_and_update_contact(profile_id, change_events)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl ProfileAuth for Entity {
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        if let Some(profile) = self.default_profile_mut() {
            profile.generate_authentication_proof(channel_state)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_authentication_proof(channel_state, responder_contact_id, proof)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl CredentialIssuer for Entity {
    fn get_signing_key(&mut self) -> Result<SecretKey> {
        if let Some(profile) = self.default_profile_mut() {
            profile.get_signing_key()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn get_signing_public_key(&mut self) -> Result<PublicKeyBytes> {
        if let Some(profile) = self.default_profile_mut() {
            profile.get_signing_public_key()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn create_offer(
        &mut self,
        schema: &CredentialSchema,
        rng: impl RngCore + CryptoRng,
    ) -> Result<CredentialOffer> {
        if let Some(profile) = self.default_profile_mut() {
            profile.create_offer(schema, rng)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn create_proof_of_possession(&mut self) -> Result<ProofBytes> {
        if let Some(profile) = self.default_profile_mut() {
            profile.create_proof_of_possession()
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<Credential> {
        if let Some(profile) = self.default_profile_mut() {
            profile.sign_credential(schema, attributes)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2> {
        if let Some(profile) = self.default_profile_mut() {
            profile.sign_credential_request(request, schema, attributes, offer_id)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl CredentialHolder for Entity {
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        public_key: PublicKeyBytes,
        rng: impl RngCore + CryptoRng,
    ) -> Result<(CredentialRequest, CredentialFragment1)> {
        if let Some(profile) = self.default_profile_mut() {
            profile.accept_credential_offer(offer, public_key, rng)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn combine_credential_fragments(
        &mut self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<Credential> {
        if let Some(profile) = self.default_profile_mut() {
            profile.combine_credential_fragments(credential_fragment1, credential_fragment2)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn is_valid_credential(
        &mut self,
        credential: &Credential,
        verifier_key: PublicKeyBytes,
    ) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.is_valid_credential(credential, verifier_key)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn present_credentials(
        &mut self,
        credential: &[Credential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
        rng: impl RngCore + CryptoRng,
    ) -> Result<Vec<CredentialPresentation>> {
        if let Some(profile) = self.default_profile_mut() {
            profile.present_credentials(credential, presentation_manifests, proof_request_id, rng)
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

impl CredentialVerifier for Entity {
    fn create_proof_request_id(&mut self, rng: impl RngCore) -> Result<ProofRequestId> {
        if let Some(profile) = self.default_profile_mut() {
            profile.create_proof_request_id(rng)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_proof_of_possession(
        &mut self,
        public_key: PublicKeyBytes,
        proof: ProofBytes,
    ) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_proof_of_possession(public_key, proof)
        } else {
            Err(ProfileNotFound.into())
        }
    }

    fn verify_credential_presentations(
        &mut self,
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        if let Some(profile) = self.default_profile_mut() {
            profile.verify_credential_presentations(
                presentations,
                presentation_manifests,
                proof_request_id,
            )
        } else {
            Err(ProfileNotFound.into())
        }
    }
}

#[async_trait]
impl CredentialProtocol for Entity {
    async fn acquire_credential(
        &mut self,
        issuer_route: Route,
        issuer_id: &ProfileIdentifier,
        schema: CredentialSchema,
    ) -> Result<EntityCredential> {
        self.ctx.send(issuer_route, IssueOfferRequest {}).await?;

        let issuer_contact = self.contacts()?.get(&issuer_id).unwrap().clone();
        let issuer_pubkey = issuer_contact.get_signing_public_key()?;
        let issuer_pubkey = issuer_pubkey.as_ref().try_into().unwrap();

        let offer = self.ctx.receive::<IssueOffer>().await?.take();
        check_message_origin(&offer, &issuer_id)?;
        let route = offer.return_route();
        // Bob accepts the credential request offer, and creates a credential request, along with the first fragment.
        let (request, frag1) =
            self.accept_credential_offer(&offer.0, issuer_pubkey, thread_rng())?;

        self.ctx.send(route, IssueRequest(request)).await?;

        let response = self.ctx.receive::<IssueResponse>().await?.take();
        check_message_origin(&response, &issuer_id)?;

        // Bob can now combine both fragments to form a Credential.
        let credential = self.combine_credential_fragments(frag1, response.body().0)?;

        // TODO: Save credential

        let credential = EntityCredential {
            credential,
            issuer_pubkey,
            schema,
        };

        Ok(credential)
    }

    async fn issue_credential(
        &mut self,
        holder_id: &ProfileIdentifier,
        schema: CredentialSchema,
    ) -> Result<()> {
        let request_offer = self
            .ctx
            .receive_timeout::<IssueOfferRequest>(120)
            .await?
            .take();
        check_message_origin(&request_offer, &holder_id)?;
        let route = request_offer.return_route();

        // The Issuer (Office) creates an Credential Request Offer (ability to open the door)
        let offer = self.create_offer(&schema, thread_rng())?;
        let offer_id = offer.id.clone();
        self.ctx.send(route, IssueOffer(offer)).await?;

        let request = self.ctx.receive_timeout::<IssueRequest>(120).await?.take();
        check_message_origin(&request_offer, &holder_id)?;
        let route = request.return_route();

        // Ask the Issuer to sign the Credential Request. A successful request results in a second fragment.
        // FIXME
        let signing_attributes = [
            (
                // TODO: ProfileIdentifier
                "door_id".into(),
                // TODO: Replace with Verifier ProfileIdentifier?
                CredentialAttribute::String("f4a8-90ff-742d-11ae".into()),
            ),
            ("can_open_door".into(), CredentialAttribute::Numeric(1)),
        ];

        // Office signs the credentials.
        let frag2 = self.sign_credential_request(
            &request.0,
            &schema,
            &(signing_attributes.clone()),
            offer_id,
        )?;

        self.ctx.send(route, IssueResponse(frag2)).await?;

        Ok(())
    }

    async fn prove_credential(
        &mut self,
        worker_route: Route,
        verifier_id: &ProfileIdentifier,
        credential: EntityCredential,
    ) -> Result<()> {
        self.ctx.send(worker_route, PresentationOffer {}).await?;

        let request_id = self
            .ctx
            .receive_timeout::<PresentationRequest>(120)
            .await?
            .take();
        check_message_origin(&request_id, &verifier_id)?;
        let route = request_id.return_route();

        let manifest = PresentationManifest {
            credential_schema: credential.schema,
            public_key: credential.issuer_pubkey,
            revealed: vec![1], // can_open_door
        };

        // Bob creates a Presentation from the manifest, his credentials, and this unique challenge instance.
        let presentation = self.present_credentials(
            &[credential.credential],
            &[manifest],
            request_id.0,
            thread_rng(),
        )?;

        self.ctx
            .send(route, PresentationResponse(presentation))
            .await?;

        Ok(())
    }
}
