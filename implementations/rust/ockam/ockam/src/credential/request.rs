use bbs::prelude::*;
use serde::{Deserialize, Serialize};

/// A request for a credential generated from a credential offer
#[derive(Debug, Deserialize, Serialize)]
pub struct CredentialRequest {
    pub(crate) context: BlindSignatureContext,
}
