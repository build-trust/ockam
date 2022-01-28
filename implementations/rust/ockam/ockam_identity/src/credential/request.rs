use crate::credential::ext::ExtBlindSignatureContext;
use crate::OfferId;
use serde::{Deserialize, Serialize};

/// A request for a credential generated from a credential offer
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    /// Offer ID sent in the credential setup.
    pub offer_id: OfferId,

    /// Context of the signature.
    pub context: ExtBlindSignatureContext,
}

#[derive(Serialize, Deserialize)]
pub enum IssuerRequest {
    GetSigningKey,
}
