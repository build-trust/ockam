use core::fmt::{Debug, Formatter};
use minicbor::{CborLen, Decode, Encode};

use crate::alloc::string::ToString;

/// Identifier length
pub const IDENTIFIER_LEN: usize = 32;

/// ChangeHash length
pub const CHANGE_HASH_LEN: usize = 32;

/// Unique identifier for an [`super::super::identity::Identity`]
/// Equals to the [`ChangeHash`] of the first [`super::Change`] in the [`super::ChangeHistory`]
/// Computed as SHA256 of the first [`super::ChangeData`] CBOR binary
#[derive(Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Encode, Decode, CborLen)]
#[cbor(transparent)]
pub struct Identifier(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; IDENTIFIER_LEN]);

impl Debug for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_string())
    }
}

/// Unique identifier for a [`super::Change`]
/// Computed as SHA256 of the corresponding [`super::ChangeData`] CBOR binary
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, CborLen)]
#[cbor(transparent)]
pub struct ChangeHash(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; CHANGE_HASH_LEN]);
