use super::structs::*;
use crate::{attribute_data::AttributeData, schema::Schema};
use bbs::prelude::*;
use serde::{Deserialize, Serialize};

/// A credential offer is how an issuer informs a potential holder that
/// a credential is available to them
#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialOffer {
    /// The credential offer id is a cryptographic nonce, this must never repeat
    pub id: [u8; 32],
    /// The schema for the credential that the issuer is offering to sign
    pub schema: Schema,
}

/// A credential that can be presented
#[derive(Debug, Serialize, Deserialize)]
pub struct Credential {
    /// The signed attributes in the credential
    pub attributes: Buffer<AttributeData>,
    /// The cryptographic signature
    pub signature: Signature,
}

/// A blind credential that will be unblinded by the holder
#[derive(Debug, Serialize, Deserialize)]
pub struct BlindCredential {
    /// The signed attributes in the credential
    pub attributes: Buffer<AttributeData>,
    /// The cryptographic signature
    pub signature: BlindSignature,
}
