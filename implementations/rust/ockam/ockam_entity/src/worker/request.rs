use crate::{
    Contact, Credential, CredentialAttribute, CredentialFragment1, CredentialFragment2,
    CredentialOffer, CredentialPresentation, CredentialPublicKey, CredentialRequest,
    CredentialSchema, KeyAttributes, OfferIdBytes, PresentationManifest, ProfileChangeEvent,
    ProfileEventAttributes, ProfileIdentifier, Proof, ProofRequestId,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProfileRequestMessage {
    Identifier,
    ChangeEvents,
    UpdateNoVerification {
        change_event: ProfileChangeEvent,
    },
    Verify,
    Contacts,
    ToContact,
    SerializeToContact,
    GetContact {
        id: ProfileIdentifier,
    },
    VerifyContact {
        contact: Contact,
    },
    VerifyAndAddContact {
        contact: Contact,
    },
    VerifyAndUpdateContact {
        profile_id: ProfileIdentifier,
        change_events: Vec<ProfileChangeEvent>,
    },
    GenerateAuthenticationProof {
        channel_state: Vec<u8>,
    },
    VerifyAuthenticationProof {
        channel_state: Vec<u8>,
        responder_contact_id: ProfileIdentifier,
        proof: Vec<u8>,
    },
    CreateKey {
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    },
    RotateKey {
        key_attributes: KeyAttributes,
        attributes: Option<ProfileEventAttributes>,
    },
    GetSecretKey {
        key_attributes: KeyAttributes,
    },
    GetPublicKey {
        key_attributes: KeyAttributes,
    },
    GetRootSecret,
    // Issuer Traits
    GetSigningKey,
    GetIssuerPublicKey,
    CreateOffer {
        schema: CredentialSchema,
    },
    CreateProofOfPossession,
    SignCredential {
        schema: CredentialSchema,
        attributes: Vec<CredentialAttribute>,
    },
    SignCredentialRequest {
        request: CredentialRequest,
        schema: CredentialSchema,
        attributes: Vec<(String, CredentialAttribute)>,
        offer_id: OfferIdBytes,
    },
    // Holder Traits
    AcceptCredentialOffer {
        offer: CredentialOffer,
        public_key: CredentialPublicKey,
    },
    CombineCredentialFragments {
        frag1: CredentialFragment1,
        frag2: CredentialFragment2,
    },
    IsValidCredential {
        credential: Credential,
        public_key: CredentialPublicKey,
    },
    PresentCredentials {
        credentials: Vec<Credential>,
        manifests: Vec<PresentationManifest>,
        proof_request_id: ProofRequestId,
    },
    // Verifier Traits
    CreateProofRequestId,
    VerifyProofOfPossession {
        public_key: CredentialPublicKey,
        proof: Proof,
    },
    VerifyCredentialPresentation {
        presentations: Vec<CredentialPresentation>,
        manifests: Vec<PresentationManifest>,
        proof_request_id: ProofRequestId,
    },
}
