use crate::credential::Verifier;
use crate::{
    BbsCredential, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialProof, CredentialPublicKey,
    CredentialRequest, CredentialRequestFragment, CredentialSchema, EntityCredential, Holder,
    Issuer, OfferId, PresentationManifest, Profile, ProofRequestId, SigningPublicKey,
};
use ockam_core::Result;
use signature_bls::SecretKey;

impl Issuer for Profile {
    fn get_signing_key(&mut self) -> Result<SecretKey> {
        self.entity().get_signing_key()
    }

    fn get_signing_public_key(&mut self) -> Result<SigningPublicKey> {
        self.entity().get_signing_public_key()
    }

    fn create_offer(&self, schema: &CredentialSchema) -> Result<CredentialOffer> {
        self.entity().create_offer(schema)
    }

    fn create_proof_of_possession(&self) -> Result<CredentialProof> {
        self.entity().create_proof_of_possession()
    }

    fn sign_credential<A: AsRef<[CredentialAttribute]>>(
        &self,
        schema: &CredentialSchema,
        attributes: A,
    ) -> Result<BbsCredential> {
        self.entity().sign_credential(schema, attributes)
    }

    fn sign_credential_request<A: AsRef<[(String, CredentialAttribute)]>>(
        &self,
        request: &CredentialRequest,
        schema: &CredentialSchema,
        attributes: A,
        offer_id: OfferId,
    ) -> Result<CredentialFragment2> {
        self.entity()
            .sign_credential_request(request, schema, attributes, offer_id)
    }
}

impl Holder for Profile {
    fn accept_credential_offer(
        &self,
        offer: &CredentialOffer,
        issuer_public_key: SigningPublicKey,
    ) -> Result<CredentialRequestFragment> {
        self.entity()
            .accept_credential_offer(offer, issuer_public_key)
    }

    fn combine_credential_fragments(
        &self,
        credential_fragment1: CredentialFragment1,
        credential_fragment2: CredentialFragment2,
    ) -> Result<BbsCredential> {
        self.entity()
            .combine_credential_fragments(credential_fragment1, credential_fragment2)
    }

    fn is_valid_credential(
        &self,
        credential: &BbsCredential,
        verifier_key: SigningPublicKey,
    ) -> Result<bool> {
        self.entity().is_valid_credential(credential, verifier_key)
    }

    fn create_credential_presentation(
        &self,
        credential: &BbsCredential,
        presentation_manifests: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<CredentialPresentation> {
        self.entity().create_credential_presentation(
            credential,
            presentation_manifests,
            proof_request_id,
        )
    }

    fn add_credential(&mut self, credential: EntityCredential) -> Result<()> {
        self.entity().add_credential(credential)
    }

    fn get_credential(&mut self, credential: &Credential) -> Result<EntityCredential> {
        self.entity().get_credential(credential)
    }
}

impl Verifier for Profile {
    fn create_proof_request_id(&self) -> Result<ProofRequestId> {
        self.entity().create_proof_request_id()
    }

    fn verify_proof_of_possession(
        &self,
        signing_public_key: CredentialPublicKey,
        proof: CredentialProof,
    ) -> Result<bool> {
        self.entity()
            .verify_proof_of_possession(signing_public_key, proof)
    }

    fn verify_credential_presentation(
        &self,
        presentation: &CredentialPresentation,
        presentation_manifest: &PresentationManifest,
        proof_request_id: ProofRequestId,
    ) -> Result<bool> {
        self.entity().verify_credential_presentation(
            presentation,
            presentation_manifest,
            proof_request_id,
        )
    }
}
