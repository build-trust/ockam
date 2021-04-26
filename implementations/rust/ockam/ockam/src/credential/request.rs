use serde::{Deserialize, Serialize};
use signature_bbs_plus::BlindSignatureContext;

/// A request for a credential generated from a credential offer
#[derive(Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    pub offer_id: [u8; 32],
    pub(crate) context: BlindSignatureContext,
}
