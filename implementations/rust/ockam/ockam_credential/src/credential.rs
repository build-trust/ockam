use super::structs::*;
use crate::{schema::Schema, serde::*};
use bbs::prelude::*;
use serde::{Deserialize, Serialize};

/// The information that
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Claim {
    /// The label associated with the claim
    pub label: ByteString,
    /// The claim type
    pub claim_type: ClaimType,
}

/// A signed value in a credential
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum ClaimType {
    /// A numeric claim
    Number(i32),
    #[serde(
        serialize_with = "write_byte_string",
        deserialize_with = "read_byte_string"
    )]
    /// A text claim
    String(ByteString),
    /// A binary claim
    Blob(Buffer<u8>),
}

/// A credential offer is how an issuer informs a potential holder that
/// a credential is available to them
#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialOffer {
    /// The credential offer id is a cryptographic nonce, this must never repeat
    pub id: [u8; 32],
    /// The claim values that will be signed
    pub claims: Buffer<Claim>,
    /// The schema for the credential that the issuer is offering to sign
    pub schema: Schema,
}

/// A credential that can be presented
#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    /// The signed attributes in the credential
    pub claims: Buffer<Claim>,
    /// The cryptographic signature
    pub signature: Signature,
}

/// A blind credential that will be unblinded by the holder
#[derive(Debug, Serialize, Deserialize)]
pub struct BlindCredential {
    /// The signed attributes in the credential
    pub claims: Buffer<Claim>,
    /// The cryptographic signature
    pub signature: BlindSignature,
}
