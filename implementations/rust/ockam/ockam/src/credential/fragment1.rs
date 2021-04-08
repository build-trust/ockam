use super::CredentialSchema;
use bbs::SignatureBlinding;
use serde::{Deserialize, Serialize};

/// The information needed to convert a CredentialFragment2 to a Credential
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialFragment1 {
    pub(crate) blinding: SignatureBlinding,
    pub(crate) schema: CredentialSchema,
}
