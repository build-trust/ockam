use crate::{
    identity::Identity, BbsCredential, BlsPublicKey, BlsSecretKey, Credential, CredentialAttribute,
    CredentialAttributeType, CredentialError, CredentialFragment1, CredentialFragment2,
    CredentialHolder, CredentialIssuer, CredentialOffer, CredentialPresentation, CredentialRequest,
    CredentialSchema, CredentialVerifier, ExtPokSignatureProof, Identity, IdentityCredential,
    IdentityError, OfferId, PresentationManifest, ProofBytes, ProofRequestId, SigningPublicKey,
};
use ockam_core::compat::collections::{HashMap, HashSet};
use ockam_core::vault::SecretVault;
use ockam_core::{allow, deny, Result};
use ockam_core::{async_trait, compat::boxed::Box};
use rand::thread_rng;
use sha2::digest::{generic_array::GenericArray, Digest, FixedOutput};
use signature_bbs_plus::{Issuer as BbsIssuer, PokSignatureProof, Prover};
use signature_bbs_plus::{MessageGenerators, ProofOfPossession};
use signature_core::challenge::Challenge;
use signature_core::lib::{HiddenMessage, Message, Nonce, ProofMessage};

impl Identity {
    pub fn add_credential(&mut self, credential: IdentityCredential) -> Result<()> {
        if let Some(_) = self
            .credentials
            .iter()
            .find(|x| x.credential() == credential.credential())
        {
            return Err(IdentityError::DuplicateCredential.into());
        }
        self.credentials.push(credential);

        Ok(())
    }

    pub fn get_credential(&mut self, credential: &Credential) -> Result<IdentityCredential> {
        if let Some(c) = self
            .credentials
            .iter()
            .find(|x| x.credential() == credential)
        {
            return Ok(c.clone());
        }

        Err(IdentityError::CredentialNotFound.into())
    }
}

#[async_trait]
impl CredentialIssuer for Identity {
    async fn get_signing_key(&mut self) -> Result<BlsSecretKey> {
        let secret = self
            .get_secret_key(Identity::CREDENTIALS_ISSUE.into())
            .await?;
        let secret = self.vault.secret_export(&secret).await?;
        let secret = BlsSecretKey::from_bytes(&secret.as_ref().try_into().unwrap()).unwrap();

        Ok(secret)
    }

    async fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        // FIXME
        let pk = BlsPublicKey::from(&self.get_signing_key().await?);
        Ok(pk.to_bytes())
    }

    async fn create_offer(&mut self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        Ok(CredentialOffer {
            id: Nonce::random(thread_rng()).to_bytes(),
            schema: schema.clone(),
        })
    }

    async fn create_proof_of_possession(&mut self) -> Result<ProofBytes> {
        Ok(ProofOfPossession::new(&self.get_signing_key().await?)
            .expect("bad signing key")
            .to_bytes())
    }

    async fn sign_credential(
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

        let generators = MessageGenerators::from_secret_key(
            &self.get_signing_key().await?,
            schema.attributes.len(),
        );

        let signature = BbsIssuer::sign(&self.get_signing_key().await?, &generators, &messages)
            .map_err(|_| CredentialError::MismatchedAttributesAndClaims)?;
        Ok(BbsCredential {
            attributes: attributes.to_vec(),
            signature,
        })
    }

    async fn sign_credential_request(
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

        let generators = MessageGenerators::from_secret_key(
            &self.get_signing_key().await?,
            schema.attributes.len(),
        );

        let signature = BbsIssuer::blind_sign(
            &request.context.clone().into(),
            &self.get_signing_key().await?,
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

#[async_trait]
impl CredentialHolder for Identity {
    async fn accept_credential_offer(
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

    async fn combine_credential_fragments(
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

    async fn is_valid_credential(
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

    async fn present_credentials(
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

#[async_trait]
impl CredentialVerifier for Identity {
    async fn create_proof_request_id(&mut self) -> Result<ProofRequestId> {
        Ok(Nonce::random(thread_rng()).to_bytes())
    }

    async fn verify_proof_of_possession(
        &mut self,
        issuer_vk: [u8; 96],
        proof: [u8; 48],
    ) -> Result<bool> {
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

    async fn verify_credential_presentations(
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
