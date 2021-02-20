use super::CredentialSchema;
use bbs::prelude::*;

/// The information needed to convert a BlindCredential to a Credential
#[derive(Debug, Clone)]
pub struct CredentialBlinding {
    pub(crate) blinding: SignatureBlinding,
    pub(crate) schema: CredentialSchema,
}
