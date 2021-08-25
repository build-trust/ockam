use crate::{
    CredentialAttribute, CredentialFragment2, CredentialOffer, CredentialPresentation,
    CredentialRequest, ProofRequestId,
};
use ockam_core::compat::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum CredentialProtocolMessage {
    IssueOfferRequest(String),
    IssueOffer(CredentialOffer),
    IssueRequest(CredentialRequest, Vec<CredentialAttribute>),
    IssueResponse(CredentialFragment2),
    PresentationOffer,
    PresentationRequest(ProofRequestId),
    PresentationResponse(CredentialPresentation),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PresentationFinishedMessage;
