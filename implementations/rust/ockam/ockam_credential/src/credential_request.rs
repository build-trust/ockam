use bbs::prelude::*;

/// A request for a credential generated from a credential offer
#[derive(Debug)]
pub struct CredentialRequest {
    pub(crate) context: BlindSignatureContext,
}
