use super::CredentialAttribute;
use serde::{Deserialize, Serialize};

/// A partial credential that will be completed by the holder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialFragment2 {
    /// The signed attributes in the credential
    pub attributes: Vec<CredentialAttribute>,
    /// The cryptographic signature
    pub signature: bbs::prelude::BlindSignature,
}
