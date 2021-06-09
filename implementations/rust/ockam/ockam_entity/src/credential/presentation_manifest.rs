use super::CredentialSchema;
use crate::PublicKeyBytes;
use ockam_core::lib::*;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! { BigArray; }

/// A list of the accepted schemas, public keys, and required to be revealed
/// attributes from a verifier
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresentationManifest {
    /// The credential schema associated with the public key
    pub credential_schema: CredentialSchema,
    #[serde(with = "BigArray")]
    /// The public key of the issuer
    pub public_key: PublicKeyBytes,
    /// The attributes required to be revealed
    pub revealed: Vec<usize>,
}
