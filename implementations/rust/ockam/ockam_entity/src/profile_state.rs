use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::{allow, deny, Result, Route};
use ockam_vault::{KeyIdVault, PublicKey, Secret, SecretAttributes};
use ockam_vault_sync_core::VaultSync;

use crate::change_history::ProfileChangeHistory;
use crate::{
    authentication::Authentication,
    profile::Profile,
    AuthenticationProof, BbsCredential, BlsPublicKey, BlsSecretKey, Changes, Contact, Contacts,
    Credential, CredentialAttribute, CredentialAttributeType, CredentialError, CredentialFragment1,
    CredentialFragment2, CredentialHolder, CredentialIssuer, CredentialOffer,
    CredentialPresentation, CredentialRequest, CredentialSchema, CredentialVerifier,
    EntityCredential, EntityError,
    EntityError::{ContactVerificationFailed, InvalidInternalState},
    EventIdentifier, ExtPokSignatureProof, Identity, KeyAttributes, Lease, MetaKeyAttributes,
    OfferId, PresentationManifest, ProfileChangeEvent, ProfileEventAttributes, ProfileIdentifier,
    ProfileVault, ProofBytes, ProofRequestId, SigningPublicKey, TTL,
};
use core::convert::TryInto;
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_vault_core::{SecretPersistence, SecretType, SecretVault, CURVE25519_SECRET_LENGTH};
use sha2::digest::{generic_array::GenericArray, Digest, FixedOutput};
use signature_bbs_plus::{Issuer as BbsIssuer, PokSignatureProof, Prover};
use signature_bbs_plus::{MessageGenerators, ProofOfPossession};
use signature_core::challenge::Challenge;
use signature_core::lib::{HiddenMessage, Message, Nonce, ProofMessage};

#[cfg(feature = "unsafe_random")]
use ockam_core::compat::rand::{thread_rng, RngCore};
#[cfg(not(feature = "unsafe_random"))]
use rand::{thread_rng, CryptoRng, RngCore};

/// Profile implementation
#[derive(Clone)]
pub struct ProfileState {
    id: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: Contacts,
    vault: VaultSync,
    rand_msg: Message,
    credentials: Vec<EntityCredential>,
    lease: Option<Lease>,
}

impl ProfileState {
    /// Profile constructor
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Changes,
        contacts: Contacts,
        vault: VaultSync,
        #[cfg(not(feature = "unsafe_random"))] rng: impl RngCore + CryptoRng + Clone,
        #[cfg(feature = "unsafe_random")] rng: impl RngCore + Clone,
    ) -> Self {
        Self {
            id: identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            vault,
            rand_msg: Message::random(rng),
            credentials: vec![],
            lease: None,
        }
    }

    pub(crate) fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }
    /// Return clone of Vault
    pub fn vault(&self) -> VaultSync {
        self.vault.clone()
    }

    /// Create ProfileState
    pub(crate) fn create(mut vault: VaultSync) -> Result<Self> {
        let initial_event_id = EventIdentifier::initial(vault.clone());

        let key_attribs = KeyAttributes::with_attributes(
            Profile::PROFILE_UPDATE.to_string(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Curve25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        );

        let create_key_event = Self::create_key_static(
            initial_event_id,
            key_attribs.clone(),
            ProfileEventAttributes::new(),
            None,
            &mut vault,
        )?;

        let create_key_change =
            ProfileChangeHistory::find_key_change_in_event(&create_key_event, &key_attribs)
                .ok_or(InvalidInternalState)?;

        let public_key = ProfileChangeHistory::get_change_public_key(&create_key_change)?;
        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;
        let public_key_id = ProfileIdentifier::from_key_id(public_key_id);

        let profile = Self::new(
            public_key_id,
            vec![create_key_event],
            Default::default(),
            vault,
            thread_rng(),
        );

        Ok(profile)
    }

    pub(crate) fn get_secret_key_from_event(
        key_attributes: &KeyAttributes,
        event: &ProfileChangeEvent,
        vault: &mut impl ProfileVault,
    ) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_public_key_from_event(key_attributes, event)?;

        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;

        vault.get_secret_by_key_id(&public_key_id)
    }

    pub fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }

    pub fn add_credential(&mut self, credential: EntityCredential) -> Result<()> {
        if let Some(_) = self
            .credentials
            .iter()
            .find(|x| x.credential() == credential.credential())
        {
            return Err(EntityError::DuplicateCredential.into());
        }
        self.credentials.push(credential);

        Ok(())
    }

    pub fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential> {
        if let Some(c) = self
            .credentials
            .iter()
            .find(|x| x.credential() == credential)
        {
            return Ok(c.clone());
        }

        Err(EntityError::CredentialNotFound.into())
    }

    pub fn has_lease(&self) -> bool {
        self.lease.is_some()
    }

    pub fn lease(&self) -> Option<&Lease> {
        self.lease.as_ref()
    }
}

impl Identity for ProfileState {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.id.clone())
    }

    fn create_key<S: Into<String>>(&mut self, label: S) -> Result<()> {
        let key_attribs = KeyAttributes::new(label.into());

        let event = { self.create_key(key_attribs, ProfileEventAttributes::new())? };
        self.add_change(event)
    }

    fn rotate_profile_key(&mut self) -> Result<()> {
        let event = {
            self.rotate_key(
                KeyAttributes::new(Profile::PROFILE_UPDATE.to_string()),
                ProfileEventAttributes::new(),
            )?
        };
        self.add_change(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_profile_secret_key(&self) -> Result<Secret> {
        self.get_secret_key(Profile::PROFILE_UPDATE)
    }

    fn get_secret_key<S: Into<String>>(&self, label: S) -> Result<Secret> {
        let key_attributes = KeyAttributes::new(label.into());
        let event = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            &key_attributes,
        )?
        .clone();
        Self::get_secret_key_from_event(&key_attributes, &event, &mut self.vault.clone())
    }

    fn get_profile_public_key(&self) -> Result<PublicKey> {
        self.get_public_key(Profile::PROFILE_UPDATE)
    }

    fn get_public_key<S: Into<String>>(&self, label: S) -> Result<PublicKey> {
        self.change_history
            .get_public_key(&KeyAttributes::new(label.into()))
    }

    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn create_auth_proof<S: AsRef<[u8]>>(
        &mut self,
        channel_state: S,
    ) -> Result<AuthenticationProof> {
        let root_secret = self.get_root_secret()?;

        Authentication::generate_proof(channel_state.as_ref(), &root_secret, &mut self.vault)
    }
    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn verify_auth_proof<S: AsRef<[u8]>, P: AsRef<[u8]>>(
        &mut self,
        channel_state: S,
        responder_contact_id: &ProfileIdentifier,
        proof: P,
    ) -> Result<bool> {
        let contact = self
            .get_contact(responder_contact_id)?
            .ok_or(EntityError::ContactNotFound)?;

        Authentication::verify_proof(
            channel_state.as_ref(),
            &contact.get_profile_update_public_key()?,
            proof.as_ref(),
            &mut self.vault,
        )
    }

    fn add_change(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        let slice = core::slice::from_ref(&change_event);
        if ProfileChangeHistory::check_consistency(self.change_history.as_ref(), &slice) {
            self.change_history.push_event(change_event);
        }
        Ok(())
    }

    fn get_changes(&self) -> Result<Changes> {
        Ok(self.change_history.as_ref().to_vec())
    }

    /// Verify whole event chain of current [`Profile`]
    fn verify_changes(&mut self) -> Result<bool> {
        if !ProfileChangeHistory::check_consistency(&[], self.change_history().as_ref()) {
            return deny();
        }

        if !self
            .change_history
            .verify_all_existing_events(&mut self.vault)?
        {
            return deny();
        }

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = self.vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if profile_id != self.identifier()? {
            return Err(EntityError::ProfileIdDoesntMatch.into());
        }

        allow()
    }

    fn get_contacts(&self) -> Result<Vec<Contact>> {
        Ok(self.contacts.values().cloned().collect())
    }

    fn as_contact(&mut self) -> Result<Contact> {
        Ok(Contact::new(
            self.id.clone(),
            self.change_history.as_ref().to_vec(),
        ))
    }

    fn get_contact(&mut self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        Ok(self.contacts.get(id).cloned())
    }

    fn verify_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        contact.verify(&mut self.vault)?;

        allow()
    }

    fn verify_and_add_contact<C: Into<Contact>>(&mut self, contact: C) -> Result<bool> {
        let contact = contact.into();
        if !self.verify_contact(contact.clone())? {
            return Err(ContactVerificationFailed.into());
        }

        self.contacts.insert(contact.identifier().clone(), contact);

        allow()
    }

    fn verify_and_update_contact<C: AsRef<[ProfileChangeEvent]>>(
        &mut self,
        contact_id: &ProfileIdentifier,
        change_events: C,
    ) -> Result<bool> {
        let contact = self
            .contacts
            .get_mut(&contact_id)
            .ok_or(EntityError::ContactNotFound)
            .expect("contact not found");

        Ok(contact.verify_and_update(change_events, &mut self.vault)?)
    }

    fn get_lease(
        &self,
        _lease_manager_route: &Route,
        _org_id: impl ToString,
        _bucket: impl ToString,
        _ttl: TTL,
    ) -> Result<Lease> {
        if let Some(lease) = self.lease.clone() {
            Ok(lease)
        } else {
            Err(InvalidInternalState.into())
        }
    }

    fn revoke_lease(&mut self, _lease_manager_route: &Route, lease: Lease) -> Result<()> {
        if let Some(existing_lease) = &self.lease {
            if existing_lease == &lease {
                self.lease = None
            }
        }
        Ok(())
    }
}

impl CredentialIssuer for ProfileState {
    fn get_signing_key(&mut self) -> Result<BlsSecretKey> {
        let secret = self.get_secret_key(Profile::CREDENTIALS_ISSUE)?;
        let secret = self.vault.secret_export(&secret)?;
        let secret = BlsSecretKey::from_bytes(&secret.as_ref().try_into().unwrap()).unwrap();

        Ok(secret)
    }

    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        // FIXME
        let pk = BlsPublicKey::from(&self.get_signing_key()?);
        Ok(pk.to_bytes())
    }

    fn create_offer(&mut self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        Ok(CredentialOffer {
            id: Nonce::random(thread_rng()).to_bytes(),
            schema: schema.clone(),
        })
    }

    fn create_proof_of_possession(&mut self) -> Result<ProofBytes> {
        Ok(ProofOfPossession::new(&self.get_signing_key()?)
            .expect("bad signing key")
            .to_bytes())
    }

    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<BbsCredential> {
        if schema.attributes.len() != attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims.into());
        }
        let mut messages = Vec::new();
        for (att, v) in schema.attributes.iter().zip(attributes) {
            match (att.attribute_type, v) {
                (CredentialAttributeType::Blob, CredentialAttribute::Blob(_)) => {
                    messages.push(v.to_signature_message())
                }
                (CredentialAttributeType::Utf8String, CredentialAttribute::String(_)) => {
                    messages.push(v.to_signature_message())
                }
                (CredentialAttributeType::Number, CredentialAttribute::Numeric(_)) => {
                    messages.push(v.to_signature_message())
                }
                (_, CredentialAttribute::NotSpecified) => messages.push(v.to_signature_message()),
                (_, CredentialAttribute::Empty) => messages.push(v.to_signature_message()),
                (_, _) => return Err(CredentialError::MismatchedAttributeClaimType.into()),
            }
        }

        let generators =
            MessageGenerators::from_secret_key(&self.get_signing_key()?, schema.attributes.len());

        let signature = BbsIssuer::sign(&self.get_signing_key()?, &generators, &messages)
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
        Ok(BbsCredential {
            attributes: attributes.to_vec(),
            signature,
        })
    }

    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferId,
    ) -> Result<CredentialFragment2> {
        if attributes.len() >= schema.attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims.into());
        }

        let mut atts = HashMap::new();

        for (name, att) in attributes {
            atts.insert(name, att);
        }

        let mut messages = Vec::<(usize, Message)>::new();
        let mut remaining_atts = Vec::<(usize, CredentialAttribute)>::new();

        // Check if any blinded messages are allowed to be unknown
        // If allowed, proceed
        // otherwise abort
        for i in 0..schema.attributes.len() {
            let att = &schema.attributes[i];
            // missing schema attribute means it's hidden by the holder
            // or unknown to the issuer
            match atts.get(&att.label) {
                None => {
                    if !att.unknown {
                        return Err(CredentialError::InvalidCredentialAttribute.into());
                    }
                }
                Some(data) => {
                    if **data != att.attribute_type {
                        return Err(CredentialError::MismatchedAttributeClaimType.into());
                    }
                    remaining_atts.push((i, (*data).clone()));
                    messages.push((i, (*data).to_signature_message()));
                }
            }
        }

        let generators =
            MessageGenerators::from_secret_key(&self.get_signing_key()?, schema.attributes.len());

        let signature = BbsIssuer::blind_sign(
            &request.context.clone().into(),
            &self.get_signing_key()?,
            &generators,
            &messages,
            Nonce::from_bytes(&offer_id).unwrap(),
        )
        .map_err(|_| CredentialError::InvalidCredentialAttribute)?;

        Ok(CredentialFragment2 {
            attributes: remaining_atts.iter().map(|(_, v)| v.clone()).collect(),
            signature,
        })
    }
}

pub const SECRET_ID: &str = "secret_id";

impl CredentialHolder for ProfileState {
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        signing_public_key: SigningPublicKey,
    ) -> Result<(CredentialRequest, CredentialFragment1)> {
        let nonce = Nonce::from_bytes(&offer.id).unwrap();
        let mut i = 0;
        let mut found = false;
        for (j, att) in offer.schema.attributes.iter().enumerate() {
            if att.label == SECRET_ID {
                i = j;
                found = true;
                break;
            }
        }
        if !found {
            return Err(CredentialError::InvalidCredentialSchema.into());
        }

        let pk = BlsPublicKey::from_bytes(&signing_public_key).unwrap();
        let generators = MessageGenerators::from_public_key(pk, offer.schema.attributes.len());
        let (context, blinding) = Prover::new_blind_signature_context(
            &[(i, self.rand_msg)],
            &generators,
            nonce,
            thread_rng(),
        )
        .map_err(|_| CredentialError::InvalidCredentialOffer)?;
        Ok((
            CredentialRequest {
                offer_id: offer.id,
                context: context.into(),
            },
            CredentialFragment1 {
                schema: offer.schema.clone(),
                blinding,
            },
        ))
    }

    fn combine_credential_fragments(
        &mut self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential> {
        let mut attributes = credential_fragment2.attributes;
        for i in 0..credential_fragment1.schema.attributes.len() {
            if credential_fragment1.schema.attributes[i].label == SECRET_ID {
                attributes.insert(i, CredentialAttribute::Blob(self.rand_msg.to_bytes()));
                break;
            }
        }
        Ok(BbsCredential {
            attributes,
            signature: credential_fragment2
                .signature
                .to_unblinded(credential_fragment1.blinding),
        })
    }

    fn is_valid_credential(
        &mut self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool> {
        // credential cannot have zero attributes so unwrap is okay
        let vk = BlsPublicKey::from_bytes(&verifier_key).unwrap();
        let generators = MessageGenerators::from_public_key(vk, credential.attributes.len());
        let msgs = credential
            .attributes
            .iter()
            .map(|a| a.to_signature_message())
            .collect::<Vec<Message>>();
        let res = credential.signature.verify(&vk, &generators, &msgs);
        Ok(res.unwrap_u8() == 1)
    }

    fn present_credentials(
        &mut self,
        credential: &[BbsCredential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<Vec<CredentialPresentation>> {
        let id_bf = Nonce::random(thread_rng());

        let mut commitments = Vec::new();
        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();

        for (cred, pm) in credential.iter().zip(presentation_manifests.iter()) {
            let mut messages = Vec::new();
            let verkey = BlsPublicKey::from_bytes(&pm.public_key).unwrap();
            let generators = MessageGenerators::from_public_key(verkey, cred.attributes.len());

            let revealed_indices = pm.revealed.iter().copied().collect::<HashSet<usize>>();
            for i in 0..cred.attributes.len() {
                if pm.credential_schema.attributes[i].label == SECRET_ID {
                    if revealed_indices.contains(&i) {
                        return Err(CredentialError::InvalidPresentationManifest.into());
                    }
                    messages.push(ProofMessage::Hidden(HiddenMessage::ExternalBlinding(
                        self.rand_msg,
                        id_bf,
                    )));
                } else if revealed_indices.contains(&i) {
                    messages.push(ProofMessage::Revealed(
                        cred.attributes[i].to_signature_message(),
                    ));
                } else {
                    messages.push(ProofMessage::Hidden(HiddenMessage::ProofSpecificBlinding(
                        cred.attributes[i].to_signature_message(),
                    )));
                }
            }

            let mut pok =
                Prover::commit_signature_pok(cred.signature, &generators, &messages, thread_rng())
                    .map_err(|_| CredentialError::MismatchedAttributeClaimType)?;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);
            pok.add_proof_contribution(&mut hasher);
            bytes = hasher.finalize();
            commitments.push(pok);
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        hasher.update(&proof_request_id);
        let challenge = Challenge::hash(&hasher.finalize());
        let presentation_id = challenge.to_bytes();

        let mut proofs = Vec::new();
        for i in 0..commitments.len() {
            let pok = commitments.remove(0);
            let cred = &credential[i];
            let pm = &presentation_manifests[i];

            let proof: ExtPokSignatureProof = pok
                .generate_proof(challenge)
                .map_err(|_| CredentialError::InvalidPresentationManifest)?
                .into();

            proofs.push(CredentialPresentation {
                presentation_id,
                revealed_attributes: pm
                    .revealed
                    .iter()
                    .map(|r| cred.attributes[*r].clone())
                    .collect(),
                proof,
            });
        }
        Ok(proofs)
    }
}

impl CredentialVerifier for ProfileState {
    fn create_proof_request_id(&mut self) -> Result<ProofRequestId> {
        Ok(Nonce::random(thread_rng()).to_bytes())
    }

    fn verify_proof_of_possession(&mut self, issuer_vk: [u8; 96], proof: [u8; 48]) -> Result<bool> {
        let public_key = BlsPublicKey::from_bytes(&issuer_vk);
        let proof = ProofOfPossession::from_bytes(&proof);

        if public_key.is_some().unwrap_u8() == 1 && proof.is_some().unwrap_u8() == 1 {
            let public_key = public_key.unwrap();
            let proof_of_possession = proof.unwrap();
            Ok(proof_of_possession.verify(public_key).unwrap_u8() == 1)
        } else {
            deny()
        }
    }

    fn verify_credential_presentations(
        &mut self,
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: [u8; 32],
    ) -> Result<bool> {
        if presentations.len() != presentation_manifests.len() || presentations.is_empty() {
            return Err(CredentialError::MismatchedPresentationAndManifests.into());
        }

        if presentations
            .iter()
            .any(|p| p.presentation_id != presentations[0].presentation_id)
        {
            return Err(CredentialError::MismatchedPresentationAndManifests.into());
        }

        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();
        let challenge = Challenge::from_bytes(&presentations[0].presentation_id).unwrap();

        for i in 0..presentations.len() {
            let prez = &presentations[i];
            let pm = &presentation_manifests[i];
            let vk = BlsPublicKey::from_bytes(&pm.public_key).unwrap();

            let proof: PokSignatureProof = prez.proof.clone().into();
            if !proof.verify(vk) {
                return deny();
            }

            let generators =
                MessageGenerators::from_public_key(vk, pm.credential_schema.attributes.len());
            let msgs = pm
                .revealed
                .iter()
                .zip(prez.revealed_attributes.iter())
                .map(|(i, r)| (*i, r.to_signature_message()))
                .collect::<Vec<(usize, Message)>>();

            let mut hasher = sha2::Sha256::new();
            hasher.update(&bytes);

            proof.add_challenge_contribution(&generators, &msgs, challenge, &mut hasher);
            bytes = hasher.finalize();
        }

        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        hasher.update(&proof_request_id);
        let challenge_verifier = Challenge::hash(&hasher.finalize());

        if challenge != challenge_verifier {
            deny()
        } else {
            allow()
        }
    }
}
