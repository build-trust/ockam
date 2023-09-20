use minicbor::{Decode, Encode};

/// Identifier length
pub const IDENTIFIER_LEN: usize = 20;

/// ChangeHash length
pub const CHANGE_HASH_LEN: usize = 20;

/// Unique identifier for an [`super::super::identity::Identity`]
/// Equals to the [`ChangeHash`] of the first [`super::Change`] in the [`super::ChangeHistory`]
/// Computed as truncated SHA256 of the first [`super::ChangeData`] CBOR binary
#[derive(Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq, Encode, Decode)]
#[cbor(transparent)]
pub struct Identifier(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; IDENTIFIER_LEN]);

/// Unique identifier for a [`super::Change`]
/// Computed as truncated SHA256 of the corresponding [`super::ChangeData`] CBOR binary
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
#[cbor(transparent)]
pub struct ChangeHash(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; CHANGE_HASH_LEN]);
