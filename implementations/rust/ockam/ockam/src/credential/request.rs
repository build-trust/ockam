use bbs::BlindSignatureContext;
use serde::{Deserialize, Serialize};

/// A request for a credential generated from a credential offer
#[derive(Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    pub offer_id: [u8; 32],
    pub(crate) context: BlindSignatureContext,
}
