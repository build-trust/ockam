use super::CredentialSchema;
use bbs::prelude::*;

/// The information needed to convert a CredentialFragment2 to a Credential
#[derive(Debug, Clone)]
pub struct CredentialFragment1 {
    pub(crate) blinding: SignatureBlinding,
    pub(crate) schema: CredentialSchema,
}
