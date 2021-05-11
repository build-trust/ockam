use serde::{Deserialize, Serialize};
use signature_bbs_plus::BlindSignatureContext;

/// A request for a credential generated from a credential offer
#[derive(Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    /// Offer ID sent in the credential setup.
    pub offer_id: [u8; 32],

    /// Context of the signature.
    pub(crate) context: BlindSignatureContext,
}
