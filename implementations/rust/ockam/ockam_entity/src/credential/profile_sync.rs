use crate::credential::traits::{CredentialHolder, CredentialIssuer, CredentialVerifier};
use crate::{
    Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialPublicKey, CredentialRequest, CredentialSchema, EntityError,
    OfferIdBytes, PresentationManifest, ProfileRequestMessage, ProfileResponseMessage, ProfileSync,
    Proof, ProofBytes, ProofRequestId, PublicKeyBytes, SigningKeyBytes,
};
use ockam_core::Result;
use ockam_node::block_future;
use rand::{CryptoRng, RngCore};

impl CredentialIssuer for ProfileSync {
    fn get_signing_key(&mut self) -> Result<SigningKeyBytes> {
        block_future(&self.ctx().runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetSigningKey)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetSigningKey(signing_key) = resp {
                Ok(signing_key)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn get_issuer_public_key(&mut self) -> Result<PublicKeyBytes> {
        block_future(&self.ctx().runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::GetIssuerPublicKey)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::GetIssuerPublicKey(public_key) = resp {
                Ok(public_key.0)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn create_offer(
        &mut self,
        schema: &CredentialSchema,
        _rng: impl RngCore + CryptoRng,
    ) -> Result<CredentialOffer> {
        block_future(&self.ctx().runtime(), async move {
            let schema = schema.clone();
            let mut ctx = self
                .send_message(ProfileRequestMessage::CreateOffer { schema })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::CreateOffer(offer) = resp {
                Ok(offer)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn create_proof_of_possession(&mut self) -> Result<ProofBytes> {
        block_future(&self.ctx().runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::CreateProofOfPossession)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::CreateProofOfPossession(proof) = resp {
                Ok(proof.0)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn sign_credential(
        &mut self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<Credential> {
        block_future(&self.ctx().runtime(), async move {
            let schema = schema.clone();
            let attributes = attributes.to_vec();
            let mut ctx = self
                .send_message(ProfileRequestMessage::SignCredential { schema, attributes })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::SignCredential(credential) = resp {
                Ok(credential)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn sign_credential_request(
        &mut self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferIdBytes,
    ) -> Result<CredentialFragment2> {
        block_future(&self.ctx().runtime(), async move {
            let request = request.clone();
            let schema = schema.clone();
            let attributes = attributes.to_vec();

            let mut ctx = self
                .send_message(ProfileRequestMessage::SignCredentialRequest {
                    request,
                    schema,
                    attributes,
                    offer_id,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::SignCredentialRequest(frag2) = resp {
                Ok(frag2)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl CredentialHolder for ProfileSync {
    fn accept_credential_offer(
        &mut self,
        offer: &CredentialOffer,
        public_key: PublicKeyBytes,
        _rng: impl RngCore + CryptoRng,
    ) -> Result<(CredentialRequest, CredentialFragment1)> {
        block_future(&self.ctx().runtime(), async move {
            let offer = offer.clone();
            let public_key = CredentialPublicKey(public_key);
            let mut ctx = self
                .send_message(ProfileRequestMessage::AcceptCredentialOffer { offer, public_key })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::AcceptCredentialOffer((request, frag1)) = resp {
                Ok((request, frag1))
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn combine_credential_fragments(
        &mut self,
        frag1: CredentialFragment1,
        frag2: CredentialFragment2,
    ) -> Result<Credential> {
        block_future(&self.ctx().runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::CombineCredentialFragments { frag1, frag2 })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::CombineCredentialFragments(credential) = resp {
                Ok(credential)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn is_valid_credential(
        &mut self,
        credential: &Credential,
        verifier_key: PublicKeyBytes,
    ) -> Result<bool> {
        block_future(&self.ctx().runtime(), async move {
            let credential = credential.clone();
            let public_key = CredentialPublicKey(verifier_key);
            let mut ctx = self
                .send_message(ProfileRequestMessage::IsValidCredential {
                    credential,
                    public_key,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::IsValidCredential(is_valid) = resp {
                Ok(is_valid)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn present_credentials(
        &mut self,
        credentials: &[Credential],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
        _rng: impl RngCore + CryptoRng,
    ) -> Result<Vec<CredentialPresentation>> {
        block_future(&self.ctx().runtime(), async move {
            let credentials = credentials.to_vec();
            let manifests = presentation_manifests.to_vec();
            let mut ctx = self
                .send_message(ProfileRequestMessage::PresentCredentials {
                    credentials,
                    manifests,
                    proof_request_id,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::PresentCredentials(presentations) = resp {
                Ok(presentations)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}

impl CredentialVerifier for ProfileSync {
    fn create_proof_request_id(&mut self, _rng: impl RngCore) -> Result<ProofRequestId> {
        block_future(&self.ctx().runtime(), async move {
            let mut ctx = self
                .send_message(ProfileRequestMessage::CreateProofRequestId)
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::CreateProofRequestId(proof_request_id) = resp {
                Ok(proof_request_id)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_proof_of_possession(
        &mut self,
        public_key: PublicKeyBytes,
        proof: ProofBytes,
    ) -> Result<bool> {
        block_future(&self.ctx().runtime(), async move {
            let public_key = CredentialPublicKey(public_key);
            let proof = Proof(proof);

            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyProofOfPossession { public_key, proof })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyProofOfPossession(has_pop) = resp {
                Ok(has_pop)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }

    fn verify_credential_presentations(
        &mut self,
        presentations: &[CredentialPresentation],
        presentation_manifests: &[PresentationManifest],
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        block_future(&self.ctx().runtime(), async move {
            let presentations = presentations.to_vec();
            let manifests = presentation_manifests.to_vec();
            let mut ctx = self
                .send_message(ProfileRequestMessage::VerifyCredentialPresentation {
                    presentations,
                    manifests,
                    proof_request_id,
                })
                .await?;

            let resp = Self::receive_message(&mut ctx).await?;

            if let ProfileResponseMessage::VerifyCredentialPresentation(has_pop) = resp {
                Ok(has_pop)
            } else {
                Err(EntityError::ProfileInvalidResponseType.into())
            }
        })
    }
}
