use super::CredentialAttribute;
use bbs::prelude::*;
use ockam_core::lib::*;
use serde::{Deserialize, Serialize};

/// Indicates how to present a credential
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CredentialPresentation {
    /// The presentation id or challenge hash
    pub presentation_id: [u8; 32],
    /// The revealed attribute values in the same canonical ordering as the presentation manifest
    pub revealed_attributes: Vec<CredentialAttribute>,
    /// The zero-knowledge proof associated with this credential
    pub proof: PoKOfSignatureProof,
}
