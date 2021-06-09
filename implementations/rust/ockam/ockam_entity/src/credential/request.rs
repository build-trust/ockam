use crate::credential::ext::ExtBlindSignatureContext;
use crate::OfferIdBytes;
use serde::{Deserialize, Serialize};

/// A request for a credential generated from a credential offer
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    /// Offer ID sent in the credential setup.
    pub offer_id: OfferIdBytes,

    /// Context of the signature.
    pub context: ExtBlindSignatureContext,
}
