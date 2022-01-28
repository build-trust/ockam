use super::CredentialAttribute;
use ockam_core::compat::vec::Vec;
use serde::{Deserialize, Serialize};
use signature_bbs_plus::BlindSignature;

/// A partial credential that will be completed by the holder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialFragment2 {
    /// The signed attributes in the credential
    pub attributes: Vec<CredentialAttribute>,
    /// The cryptographic signature
    pub signature: BlindSignature,
}
