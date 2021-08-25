// TODO restore #![deny(missing_docs)]

use ockam_core::compat::{string::String, vec::Vec};
use serde_big_array::big_array;

big_array! { BigArray; }

/// Alias for an array of 32 bytes.
pub type SigningKey = [u8; 32];

/// Alias for an array of 32 bytes.
pub type PresentationIdBytes = [u8; 32];

/// Alias for an array of 96 bytes.
pub type SigningPublicKey = [u8; 96];

/// Serializable wrapper around a credential public key.
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct CredentialPublicKey(#[serde(with = "BigArray")] pub SigningPublicKey);

/// Alias for an array of 48 bytes.
pub type ProofBytes = [u8; 48];

/// Serializable wrapper around a proof.
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct CredentialProof(#[serde(with = "BigArray")] pub ProofBytes);

/// Alias for an array of Nonce::BYTES length.
pub type ProofRequestId = [u8; Nonce::BYTES];

/// Alias for an array of Nonce::BYTES length.
pub type OfferId = [u8; Nonce::BYTES];

mod attribute;
mod attribute_schema;
mod attribute_type;
mod error;
mod ext;
mod fragment1;
mod fragment2;
mod macros;
mod messages;
mod offer;
mod presentation;
mod presentation_manifest;
mod request;
mod schema;
mod traits;
mod util;
mod workers;

pub use attribute::*;
pub use attribute_schema::*;
pub use attribute_type::*;
pub use error::*;
pub use ext::*;
pub use fragment1::*;
pub use fragment2::*;
pub use macros::*;
pub(crate) use messages::*;
pub use offer::*;
pub use presentation::*;
pub use presentation_manifest::*;
pub use request::*;
pub use schema::*;
use serde::{Deserialize, Serialize};
use signature_bbs_plus::Signature;
use signature_core::nonce::Nonce;
pub use traits::*;
use util::*;
pub(crate) use workers::*;

/// A credential that can be presented
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BbsCredential {
    /// The signed attributes in the credential
    pub attributes: Vec<CredentialAttribute>,
    /// The cryptographic signature
    pub signature: Signature,
}

/// A credential and fragment 1 pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialRequestFragment(pub CredentialRequest, pub CredentialFragment1);
