use crate::CredentialPresentation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IssueOfferRequest;

#[derive(Serialize, Deserialize)]
pub struct IssueOffer(pub crate::CredentialOffer);

#[derive(Serialize, Deserialize)]
pub struct IssueRequest(pub crate::CredentialRequest);

#[derive(Serialize, Deserialize)]
pub struct IssueResponse(pub crate::CredentialFragment2);

#[derive(Serialize, Deserialize)]
pub struct PresentationOffer;

#[derive(Serialize, Deserialize)]
pub struct PresentationRequest(pub [u8; 32]);

#[derive(Serialize, Deserialize)]
pub struct PresentationResponse(pub Vec<CredentialPresentation>);
