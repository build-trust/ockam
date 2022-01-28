use crate::{
    Credential, CredentialAttribute, CredentialFragment2, CredentialOffer, CredentialPresentation,
    CredentialRequest, ProofRequestId,
};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::Message;
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

#[derive(Serialize, Deserialize)]
pub(crate) struct CredentialAcquisitionResultMessage {
    pub(crate) credential: Credential,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CredentialVerificationResultMessage {
    pub(crate) is_valid: bool,
}

impl Message for CredentialProtocolMessage {}

impl Message for PresentationFinishedMessage {}

impl Message for CredentialAcquisitionResultMessage {}

impl Message for CredentialVerificationResultMessage {}
