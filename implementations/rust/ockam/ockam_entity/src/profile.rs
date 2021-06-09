use crate::authentication::Authentication;
use crate::credential::CredentialVerifier;
use crate::history::ProfileChangeHistory;
use crate::{
    Contact, ContactsDb, Credential, CredentialAttribute, CredentialAttributeType, CredentialError,
    CredentialFragment1, CredentialFragment2, CredentialHolder, CredentialIssuer, CredentialOffer,
    CredentialPresentation, CredentialRequest, CredentialSchema, EntityError, EventIdentifier,
    KeyAttributes, OfferIdBytes, PresentationManifest, Profile, ProfileAuth, ProfileChangeEvent,
    ProfileChanges, ProfileContacts, ProfileEventAttributes, ProfileIdentifier, ProfileIdentity,
    ProfileSecrets, ProfileVault, ProofBytes, ProofRequestId, PublicKeyBytes, SigningKeyBytes,
    SECRET_ID,
};

use ockam_core::Result;
use ockam_vault_core::{PublicKey, Secret};
use rand::{thread_rng, CryptoRng, RngCore};
use sha2::digest::generic_array::GenericArray;
use sha2::digest::FixedOutput;
use sha2::Digest;
use signature_bbs_plus::{
    BlindSignatureContext, Issuer as BbsIssuer, MessageGenerators, PokSignatureProof, Prover,
    PublicKey as BbsPublicKey,
};
use signature_bls::{ProofOfPossession, SecretKey};
use signature_core::challenge::Challenge;
use signature_core::hidden_message::HiddenMessage;
use signature_core::lib::HashSet;
use signature_core::message::Message;
use signature_core::nonce::Nonce;
use signature_core::proof_message::ProofMessage;
use std::collections::HashMap;

/// Profile implementation
pub struct ProfileImpl<V: ProfileVault> {
    identifier: ProfileIdentifier,
    change_history: ProfileChangeHistory,
    contacts: ContactsDb,
    signing_key: SecretKey,
    signing_id_message: Message,
    pub(crate) vault: V,
}

impl<V: ProfileVault> ProfileImpl<V> {
    /// Profile constructor
    pub fn new(
        identifier: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
        contacts: ContactsDb,
        credential_issuing_key: SecretKey,
        vault: V,
    ) -> Self {
        Self {
            identifier,
            change_history: ProfileChangeHistory::new(change_events),
            contacts,
            signing_key: credential_issuing_key,
            signing_id_message: Message::random(thread_rng()),
            vault,
        }
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    pub(crate) fn change_history(&self) -> &ProfileChangeHistory {
        &self.change_history
    }
    /// Return clone of Vault
    pub fn vault(&self) -> V {
        self.vault.clone()
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    /// Generate fresh [`Profile`] update key and create new [`Profile`] using it
    pub(crate) fn create_internal(
        attributes: Option<ProfileEventAttributes>,
        credential_issuing_key: SecretKey,
        mut vault: V,
    ) -> Result<Self> {
        let prev_id = vault.sha256(Profile::NO_EVENT)?;
        let prev_id = EventIdentifier::from_hash(prev_id);

        let key_attributes = KeyAttributes::new(Profile::PROFILE_UPDATE.to_string());
        let change_event = Self::create_key_event_static(
            prev_id,
            key_attributes.clone(),
            attributes,
            None,
            &mut vault,
        )?;

        let change = ProfileChangeHistory::find_key_change_in_event(&change_event, &key_attributes)
            .ok_or(EntityError::InvalidInternalState)?;
        let public_key = ProfileChangeHistory::get_change_public_key(&change)?;

        let public_key_id = vault.compute_key_id_for_public_key(&public_key)?;
        let public_key_id = ProfileIdentifier::from_key_id(public_key_id);

        let profile = Self::new(
            public_key_id,
            vec![change_event],
            Default::default(),
            credential_issuing_key,
            vault,
        );

        Ok(profile)
    }
}

impl<V: ProfileVault> ProfileImpl<V> {
    pub(crate) fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history.as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
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
}

impl<V: ProfileVault> ProfileIdentity for ProfileImpl<V> {
    fn identifier(&self) -> Result<ProfileIdentifier> {
        Ok(self.identifier.clone())
    }
}

impl<V: ProfileVault> ProfileChanges for ProfileImpl<V> {
    fn change_events(&self) -> Result<Vec<ProfileChangeEvent>> {
        Ok(self.change_history.as_ref().to_vec())
    }
    fn update_no_verification(&mut self, change_event: ProfileChangeEvent) -> Result<()> {
        let slice = std::slice::from_ref(&change_event);
        ProfileChangeHistory::check_consistency(self.change_history.as_ref(), &slice)?;
        self.change_history.push_event(change_event);

        Ok(())
    }
    /// Verify whole event chain of current [`Profile`]
    fn verify(&mut self) -> Result<bool> {
        ProfileChangeHistory::check_consistency(&[], self.change_history().as_ref())?;

        self.change_history
            .verify_all_existing_events(&mut self.vault)?;

        let root_public_key = self.change_history.get_first_root_public_key()?;

        let root_key_id = self.vault.compute_key_id_for_public_key(&root_public_key)?;
        let profile_id = ProfileIdentifier::from_key_id(root_key_id);

        if profile_id != self.identifier()? {
            return Err(EntityError::ProfileIdDoesntMatch.into());
        }

        Ok(true)
    }
}

impl<V: ProfileVault> ProfileSecrets for ProfileImpl<V> {
    fn create_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        let event = {
            let root_secret = self.get_root_secret()?;
            self.create_key_event(key_attributes, attributes, Some(&root_secret))?
        };
        self.update_no_verification(event)
    }

    fn rotate_key(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    ) -> Result<()> {
        let event = {
            let root_secret = self.get_root_secret()?;
            self.rotate_key_event(key_attributes, attributes, &root_secret)?
        };
        self.update_no_verification(event)
    }

    /// Get [`Secret`] key. Key is uniquely identified by label in [`KeyAttributes`]
    fn get_secret_key(&mut self, key_attributes: &KeyAttributes) -> Result<Secret> {
        let event = ProfileChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            key_attributes,
        )?
        .clone();
        Self::get_secret_key_from_event(key_attributes, &event, &mut self.vault)
    }

    fn get_public_key(&self, key_attributes: &KeyAttributes) -> Result<PublicKey> {
        self.change_history.get_public_key(key_attributes)
    }
    fn get_root_secret(&mut self) -> Result<Secret> {
        let public_key = ProfileChangeHistory::get_current_profile_update_public_key(
            self.change_history().as_ref(),
        )?;

        let key_id = self.vault.compute_key_id_for_public_key(&public_key)?;
        self.vault.get_secret_by_key_id(&key_id)
    }
}

impl<V: ProfileVault> ProfileContacts for ProfileImpl<V> {
    fn contacts(&self) -> Result<ContactsDb> {
        Ok(self.contacts.clone())
    }

    fn to_contact(&self) -> Result<Contact> {
        Ok(Contact::new(
            self.identifier.clone(),
            self.change_history.as_ref().to_vec(),
        ))
    }

    fn serialize_to_contact(&self) -> Result<Vec<u8>> {
        let contact = self.to_contact()?;

        Profile::serialize_contact(&contact)
    }

    fn get_contact(&self, id: &ProfileIdentifier) -> Result<Option<Contact>> {
        Ok(self.contacts.get(id).cloned())
    }

    fn verify_contact(&mut self, contact: &Contact) -> Result<bool> {
        contact.verify(&mut self.vault)?;

        Ok(true)
    }

    fn verify_and_add_contact(&mut self, contact: Contact) -> Result<bool> {
        self.verify_contact(&contact)?;

        let _ = self.contacts.insert(contact.identifier().clone(), contact);

        Ok(true)
    }

    fn verify_and_update_contact(
        &mut self,
        profile_id: &ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    ) -> Result<bool> {
        let contact = self
            .contacts
            .get_mut(profile_id)
            .ok_or(EntityError::ContactNotFound)?;

        contact.verify_and_update(change_events, &mut self.vault)?;

        Ok(true)
    }
}

impl<V: ProfileVault> ProfileAuth for ProfileImpl<V> {
    /// Generate Proof of possession of [`Profile`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn generate_authentication_proof(&mut self, channel_state: &[u8]) -> Result<Vec<u8>> {
        let root_secret = self.get_root_secret()?;

        Authentication::generate_proof(channel_state, &root_secret, &mut self.vault)
    }

    /// Verify Proof of possession of [`Profile`] with given [`ProfileIdentifier`].
    /// channel_state should be tied to channel's cryptographical material (e.g. h value for Noise XX)
    fn verify_authentication_proof(
        &mut self,
        channel_state: &[u8],
        responder_contact_id: &ProfileIdentifier,
        proof: &[u8],
    ) -> Result<bool> {
        let contact = self
            .get_contact(responder_contact_id)?
            .ok_or(EntityError::ContactNotFound)?;

        Authentication::verify_proof(
            channel_state,
            &contact.get_profile_update_public_key()?,
            proof,
            &mut self.vault,
        )
    }
}

impl<V: ProfileVault> CredentialIssuer for ProfileImpl<V> {
    /// Return the signing key associated with this CredentialIssuer
    fn get_signing_key(&mut self) -> Result<SigningKeyBytes> {
        Ok(self.signing_key.to_bytes())
    }

    /// Return the public key
    fn get_issuer_public_key(&mut self) -> Result<PublicKeyBytes> {
        let pk = BbsPublicKey::from(&self.signing_key);
        Ok(pk.to_bytes())
    }

    /// Create a credential offer
    fn create_offer(
        &mut self,
        schema: &CredentialSchema,
        rng: impl RngCore + CryptoRng,
    ) -> Result<CredentialOffer> {
        let id = Nonce::random(rng).to_bytes();
        Ok(CredentialOffer {
            id,
            schema: schema.clone(),
        })
    }

    /// Create a proof of possession for this issuers signing key
    fn create_proof_of_possession(&mut self) -> Result<ProofBytes> {
        Ok(ProofOfPossession::new(&self.signing_key)
            .expect("bad signing key")
            .to_bytes())
    }

    /// Sign the claims into the credential
    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<Credential> {
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
            MessageGenerators::from_secret_key(&self.signing_key, schema.attributes.len());

        let signature = BbsIssuer::sign(&self.signing_key, &generators, &messages)
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
        Ok(Credential {
            attributes: attributes.to_vec(),
            signature,
        })
    }

    /// Sign a credential request where certain claims have already been committed and signs the remaining claims
    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2> {
        if attributes.len() >= schema.attributes.len() {
            return Err(CredentialError::MismatchedAttributesAndClaims.into());
        }

        let mut request_attributes = HashMap::new();

        for (request_attribute_name, request_attribute) in attributes {
            request_attributes.insert(request_attribute_name, request_attribute);
        }

        let mut messages = Vec::<(usize, Message)>::new();
        let mut remaining_attributes = Vec::<(usize, CredentialAttribute)>::new();

        // Check if any blinded messages are allowed to be unknown
        // If allowed, proceed
        // otherwise abort
        for schema_attribute_index in 0..schema.attributes.len() {
            let schema_attribute = &schema.attributes[schema_attribute_index];
            // missing schema attribute means it's hidden by the holder
            // or unknown to the issuer
            match request_attributes.get(&schema_attribute.label) {
                None => {
                    if !schema_attribute.unknown {
                        return Err(CredentialError::InvalidCredentialAttribute.into());
                    }
                }
                Some(attribute) => {
                    if **attribute != schema_attribute.attribute_type {
                        return Err(CredentialError::MismatchedAttributeClaimType.into());
                    }
                    remaining_attributes.push((schema_attribute_index, (*attribute).clone()));
                    messages.push((schema_attribute_index, (*attribute).to_signature_message()));
                }
            }
        }

        let generators =
            MessageGenerators::from_secret_key(&self.signing_key, schema.attributes.len());

        let ctx: BlindSignatureContext = request.context.clone().into();
        let signature = BbsIssuer::blind_sign(
            &ctx,
            &self.signing_key,
            &generators,
            &messages,
            Nonce::from_bytes(&offer_id).unwrap(),
        )
        .map_err(|_| CredentialError::InvalidCredentialAttribute)?;

        Ok(CredentialFragment2 {
            attributes: remaining_attributes
                .iter()
                .map(|(_, v)| v.clone())
                .collect(),
            signature,
        })
    }
}

impl<V: ProfileVault> CredentialHolder for ProfileImpl<V> {
    /// Accepts a credential offer from an issuer
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        issuer_pk: PublicKeyBytes,
        rng: impl RngCore + CryptoRng,
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

        let pk = BbsPublicKey::from_bytes(&issuer_pk).unwrap();
        let generators = MessageGenerators::from_public_key(pk, offer.schema.attributes.len());

        let (context, blinding) = Prover::new_blind_signature_context(
            &[(i, self.signing_id_message)],
            &generators,
            nonce,
            rng,
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

    /// Combine credential fragments to yield a completed credential
    fn combine_credential_fragments(
        &mut self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<Credential> {
        let mut attributes = credential_fragment2.attributes;
        let signing_id = self.signing_id_message;

        for i in 0..credential_fragment1.schema.attributes.len() {
            if credential_fragment1.schema.attributes[i].label == SECRET_ID {
                attributes.insert(i, CredentialAttribute::Blob(signing_id.to_bytes()));
                break;
            }
        }
        Ok(Credential {
            attributes,
            signature: credential_fragment2
                .signature
                .to_unblinded(credential_fragment1.blinding),
        })
    }

    /// Check a credential to make sure its valid
    fn is_valid_credential(
        &mut self,
        credential: &Credential,
        verifier_key: PublicKeyBytes,
    ) -> Result<bool> {
        // credential cannot have zero attributes so unwrap is okay
        let vk = BbsPublicKey::from_bytes(&verifier_key).unwrap();
        let generators = MessageGenerators::from_public_key(vk, credential.attributes.len());
        let msgs = credential
            .attributes
            .iter()
            .map(|a| a.to_signature_message())
            .collect::<Vec<Message>>();
        let res = credential.signature.verify(&vk, &generators, &msgs);
        Ok(res.unwrap_u8() == 1)
    }

    /// Given a list of credentials, and a list of manifests
    /// generates a zero-knowledge presentation.
    ///
    /// Each credential maps to a presentation manifest
    fn present_credentials(
        &mut self,
        credential: &[Credential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
        mut rng: impl RngCore + CryptoRng,
    ) -> Result<Vec<CredentialPresentation>> {
        // To prove the id-secret is the same across credentials we use a Schnorr proof
        // which requires that the proof blinding factor be the same. If there's only one credential
        // it makes no difference
        let id_bf = Nonce::random(&mut rng);

        let mut commitments = Vec::new();
        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();

        for (cred, pm) in credential.iter().zip(presentation_manifests.iter()) {
            let mut messages = Vec::new();
            let verkey = BbsPublicKey::from_bytes(&pm.public_key).unwrap();
            let generators = MessageGenerators::from_public_key(verkey, cred.attributes.len());
            // let pr = bbs::prelude::Verifier::new_proof_request(pm.revealed.as_slice(), &verkey)
            //     .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;

            let revealed_indices = pm.revealed.iter().copied().collect::<HashSet<usize>>();

            let signing_id = self.signing_id_message;

            for i in 0..cred.attributes.len() {
                if pm.credential_schema.attributes[i].label == SECRET_ID {
                    if revealed_indices.contains(&i) {
                        return Err(CredentialError::InvalidPresentationManifest.into());
                    }
                    messages.push(ProofMessage::Hidden(HiddenMessage::ExternalBlinding(
                        signing_id, id_bf,
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
                Prover::commit_signature_pok(cred.signature, &generators, &messages, &mut rng)
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

            let proof = pok
                .generate_proof(challenge)
                .map_err(|_| CredentialError::InvalidPresentationManifest)?;

            proofs.push(CredentialPresentation {
                presentation_id,
                revealed_attributes: pm
                    .revealed
                    .iter()
                    .map(|r| cred.attributes[*r].clone())
                    .collect(),
                proof: proof.into(),
            });
        }

        Ok(proofs)
    }
}

impl<V: ProfileVault> CredentialVerifier for ProfileImpl<V> {
    /// Create a unique proof request id so the holder must create a fresh proof
    fn create_proof_request_id(&mut self, rng: impl RngCore) -> Result<ProofRequestId> {
        Ok(Nonce::random(rng).to_bytes())
    }

    /// Verify a proof of possession
    fn verify_proof_of_possession(
        &mut self,
        issuer_verifying_key: PublicKeyBytes,
        proof: ProofBytes,
    ) -> Result<bool> {
        let vk = BbsPublicKey::from_bytes(&issuer_verifying_key);
        let proof = ProofOfPossession::from_bytes(&proof);

        Ok(
            if vk.is_some().unwrap_u8() == 1 && proof.is_some().unwrap_u8() == 1 {
                let pk = vk.unwrap();
                let p = proof.unwrap();
                p.verify(pk).unwrap_u8() == 1
            } else {
                false
            },
        )
    }

    /// Check if the credential presentations are valid
    fn verify_credential_presentations(
        &mut self,
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        if presentations.len() != presentation_manifests.len() || presentations.is_empty() {
            return Ok(false); // Err(CredentialError::MismatchedPresentationAndManifests.into());
        }

        if presentations
            .iter()
            .any(|p| p.presentation_id != presentations[0].presentation_id)
        {
            return Ok(false); // Err(CredentialError::MismatchedPresentationAndManifests.into());
        }

        let mut bytes = GenericArray::<u8, <sha2::Sha256 as FixedOutput>::OutputSize>::default();
        let challenge = Challenge::from_bytes(&presentations[0].presentation_id).unwrap();

        for i in 0..presentations.len() {
            let prez = &presentations[i];
            let pm = &presentation_manifests[i];
            let vk = BbsPublicKey::from_bytes(&pm.public_key).unwrap();

            let proof: PokSignatureProof = prez.proof.clone().into();

            if !proof.verify(vk) {
                return Ok(false); // Err(CredentialError::InvalidCredentialPresentation(i as u32).into());
            }

            let generators =
                MessageGenerators::from_public_key(vk, pm.credential_schema.attributes.len());
            let msgs = pm
                .revealed
                .iter()
                .zip(prez.revealed_attributes.iter())
                .map(|(i, r)| (*i, r.to_signature_message()))
                .collect::<heapless::Vec<(usize, Message), 64>>();

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
            return Ok(false); // Err(CredentialError::InvalidPresentationChallenge.into());
        }

        Ok(true)
    }
}
