use crate::credential::Verifier;
use crate::{
    BbsCredential, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialProof, CredentialPublicKey,
    CredentialRequest, CredentialRequestFragment, CredentialSchema, EntityCredential, Holder,
    Issuer, OfferId, PresentationManifest, Profile, ProofRequestId, SigningPublicKey,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use signature_bls::SecretKey;

#[async_trait]
impl Issuer for Profile {
    async fn get_signing_key(&mut self) -> Result<SecretKey> {
        self.entity().await?.get_signing_key().await
    }

    async fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        self.entity().await?.get_signing_public_key().await
    }

    async fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        self.entity().await?.create_offer(schema).await
    }

    async fn create_proof_of_possession(&self) -> Result<CredentialProof> {
        self.entity().await?.create_proof_of_possession().await
    }

    async fn sign_credential(
        &self,
        schema: &CredentialSchema,
        attributes: &[CredentialAttribute],
    ) -> Result<BbsCredential> {
        self.entity()
            .await?
            .sign_credential(schema, attributes)
            .await
    }

    async fn sign_credential_request(
        &self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: &[(String, CredentialAttribute)],
        offer_id: OfferId,
    ) -> Result<CredentialFragment2> {
        self.entity()
            .await?
            .sign_credential_request(request, schema, attributes, offer_id)
            .await
    }
}

#[async_trait]
impl Holder for Profile {
    async fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_public_key: SigningPublicKey,
    ) -> Result<CredentialRequestFragment> {
        self.entity()
            .await?
            .accept_credential_offer(offer, issuer_public_key)
            .await
    }

    async fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential> {
        self.entity()
            .await?
            .combine_credential_fragments(credential_fragment1, credential_fragment2)
            .await
    }

    async fn is_valid_credential(
        &self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool> {
        self.entity()
            .await?
            .is_valid_credential(credential, verifier_key)
            .await
    }

    async fn create_credential_presentation(
        &self,
        credential: &BbsCredential,
        presentation_manifests: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<CredentialPresentation> {
        self.entity()
            .await?
            .create_credential_presentation(credential, presentation_manifests, proof_request_id)
            .await
    }

    async fn add_credential(&mut self, credential: EntityCredential) -> Result<()> {
        self.entity().await?.add_credential(credential).await
    }

    async fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential> {
        self.entity().await?.get_credential(credential).await
    }
}

#[async_trait]
impl Verifier for Profile {
    async fn create_proof_request_id(&self) -> Result<ProofRequestId> {
        self.entity().await?.create_proof_request_id().await
    }

    async fn verify_proof_of_possession(
        &self,
        signing_public_key: CredentialPublicKey,
        proof: CredentialProof,
    ) -> Result<bool> {
        self.entity()
            .await?
            .verify_proof_of_possession(signing_public_key, proof)
            .await
    }

    async fn verify_credential_presentation(
        &self,
        presentation: &CredentialPresentation,
        presentation_manifest: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        self.entity()
            .await?
            .verify_credential_presentation(presentation, presentation_manifest, proof_request_id)
            .await
    }
}
