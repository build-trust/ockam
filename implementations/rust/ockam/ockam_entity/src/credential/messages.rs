use crate::CredentialPresentation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum CredentialProtocolMessage {
    IssueOfferRequest,
    IssueOffer(crate::CredentialOffer),
    IssueRequest(crate::CredentialRequest),
    IssueResponse(crate::CredentialFragment2),
    PresentationOffer,
    PresentationRequest([u8; 32]),
    PresentationResponse(CredentialPresentation),
}
