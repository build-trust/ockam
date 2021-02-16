use crate::CredentialAttribute;
use bbs::prelude::*;
use ockam_core::lib::*;

/// Indicates how to present a credential
#[derive(Debug, Clone)]
pub struct CredentialPresentation {
    /// The revealed attribute values in the same canonical ordering as the presentation manifest
    pub revealed_attributes: Vec<CredentialAttribute>,
    /// The zero-knowledge proof associated with this credential
    pub proof: PoKOfSignatureProof,
}
