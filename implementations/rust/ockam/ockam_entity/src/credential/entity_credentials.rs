use crate::credential::Verifier;
use crate::EntityError::IdentityApiFailed;
use crate::IdentityCredentialRequest::*;
use crate::{
    BbsCredential, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialProof, CredentialProtocol,
    CredentialPublicKey, CredentialRequest, CredentialRequestFragment, CredentialSchema, Entity,
    EntityCredential, Holder, HolderWorker, Identity, IdentityCredentialResponse, IdentityRequest,
    IdentityResponse, Issuer, ListenerWorker, OfferId, PresentationFinishedMessage,
    PresentationManifest, PresenterWorker, ProfileIdentifier, ProofRequestId, SigningPublicKey,
    TrustPolicy, TrustPolicyImpl, VerifierWorker,
};
use core::convert::TryInto;
use ockam_core::{Address, Result, Route};
use ockam_node::block_future;
use signature_bls::SecretKey;
use IdentityRequest::*;
use IdentityResponse as Res;

fn err<T>() -> Result<T> {
    Err(IdentityApiFailed.into())
}

impl Issuer for Entity {
    fn get_signing_key(&mut self) -> Result<SecretKey> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(GetSigningKey(
            profile.identifier().expect("couldn't get profile id"),
        )))? {
            if let IdentityCredentialResponse::GetSigningKey(signing_key) = res {
                Ok(signing_key)
            } else {
                err()
            }
        } else {
            err()
        }
    }

    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        // FIXME: Why clone?
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(
            GetIssuerPublicKey(profile.identifier().expect("couldn't get profile id")),
        ))? {
            if let IdentityCredentialResponse::GetIssuerPublicKey(issuer_public_key) = res {
                Ok(issuer_public_key.0)
            } else {
                err()
            }
        } else {
            err()
        }
    }

    fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(CreateOffer(
            profile.identifier().expect("couldn't get profile id"),
            schema.clone(),
        )))? {
            if let IdentityCredentialResponse::CreateOffer(offer) = res {
                Ok(offer)
            } else {
                err()
            }
        } else {
            err()
        }
    }

    fn create_proof_of_possession(&self) -> Result<CredentialProof> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(
            CreateProofOfPossession(profile.identifier().expect("couldn't get profile id")),
        ))? {
            if let IdentityCredentialResponse::CreateProofOfPossession(proof) = res {
                Ok(proof)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(SignCredential(
            profile.identifier().expect("couldn't get profile id"),
            schema.clone(),
            attributes.as_ref().to_vec(),
        )))? {
            if let IdentityCredentialResponse::SignCredential(credential) = res {
                Ok(credential)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) =
            profile.call(CredentialRequest(SignCredentialRequest(
                profile.identifier().expect("couldn't get profile id"),
                request.clone(),
                schema.clone(),
                attributes.as_ref().to_vec(),
                offer_id,
            )))?
        {
            if let IdentityCredentialResponse::SignCredentialRequest(frag) = res {
                Ok(frag)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) =
            profile.call(CredentialRequest(AcceptCredentialOffer(
                profile.identifier().expect("couldn't get profile id"),
                offer.clone(),
                CredentialPublicKey(issuer_public_key),
            )))?
        {
            if let IdentityCredentialResponse::AcceptCredentialOffer(request_fragment) = res {
                Ok(request_fragment)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) =
            profile.call(CredentialRequest(CombineCredentialFragments(
                profile.identifier().expect("couldn't get profile id"),
                credential_fragment1,
                credential_fragment2,
            )))?
        {
            if let IdentityCredentialResponse::CombineCredentialFragments(credential) = res {
                Ok(credential)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(IsValidCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
            CredentialPublicKey(verifier_key),
        )))? {
            if let IdentityCredentialResponse::IsValidCredential(valid) = res {
                Ok(valid)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(PresentCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
            presentation_manifests.clone(),
            proof_request_id,
        )))? {
            if let IdentityCredentialResponse::PresentCredential(presentation) = res {
                Ok(presentation)
            } else {
                err()
            }
        } else {
            err()
        }
    }

    fn add_credential(&mut self, credential: EntityCredential) -> Result<()> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(AddCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential,
        )))? {
            if let IdentityCredentialResponse::AddCredential = res {
                Ok(())
            } else {
                err()
            }
        } else {
            err()
        }
    }

    fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(GetCredential(
            profile.identifier().expect("couldn't get profile id"),
            credential.clone(),
        )))? {
            if let IdentityCredentialResponse::GetCredential(c) = res {
                Ok(c)
            } else {
                err()
            }
        } else {
            err()
        }
    }
}

impl Verifier for Entity {
    fn create_proof_request_id(&self) -> Result<ProofRequestId> {
        let profile = self.clone().current_profile().expect("no current profile");
        if let Res::CredentialResponse(res) = profile.call(CredentialRequest(
            CreateProofRequestId(profile.identifier().expect("couldn't get profile id")),
        ))? {
            if let IdentityCredentialResponse::CreateProofRequestId(request_id) = res {
                Ok(request_id)
            } else {
                err()
            }
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

        if let Res::CredentialResponse(res) =
            profile.call(CredentialRequest(VerifyProofOfPossession(
                profile.identifier().expect("couldn't get profile id"),
                signing_public_key,
                proof,
            )))?
        {
            if let IdentityCredentialResponse::VerifyProofOfPossession(verified) = res {
                Ok(verified)
            } else {
                err()
            }
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
        if let Res::CredentialResponse(res) =
            profile.call(CredentialRequest(VerifyCredentialPresentation(
                profile.identifier().expect("couldn't get profile id"),
                presentation.clone(),
                presentation_manifest.clone(),
                proof_request_id,
            )))?
        {
            if let IdentityCredentialResponse::VerifyCredentialPresentation(verified) = res {
                Ok(verified)
            } else {
                err()
            }
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
