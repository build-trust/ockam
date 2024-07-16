use crate::compat::borrow::Cow;
use crate::compat::vec::Vec;

use core::ops::Deref;
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};

/// A new type around `Cow<'_, [u8]>` that borrows from input.
///
/// Contrary to `Cow<_, [u8]>` the `Decode` impl for this type will always borrow
/// from input so using it in types like `Option`, `Vec<_>` etc will not produce
/// owned element values.
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    CborLen,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[cbor(transparent)]
#[serde(transparent)]
pub struct CowBytes<'a>(
    #[cbor(b(0), with = "minicbor::bytes")]
    #[serde(borrow)]
    pub Cow<'a, [u8]>,
);

impl CowBytes<'_> {
    /// Returns true if the data is borrowed, i.e. if to_mut would require additional work.
    pub fn is_borrowed(&self) -> bool {
        matches!(self.0, Cow::Borrowed(_))
    }

    /// Create and return owned CowBytes
    pub fn to_owned<'r>(&self) -> CowBytes<'r> {
        CowBytes(Cow::Owned(self.0.to_vec()))
    }

    /// Turn into owned CowBytes
    pub fn into_owned(self) -> Vec<u8> {
        self.0.into_owned()
    }
}

impl<'a> From<&'a [u8]> for CowBytes<'a> {
    fn from(s: &'a [u8]) -> Self {
        CowBytes(Cow::Borrowed(s))
    }
}

impl From<Vec<u8>> for CowBytes<'_> {
    fn from(s: Vec<u8>) -> Self {
        CowBytes(Cow::Owned(s))
    }
}

impl<'a> From<CowBytes<'a>> for Cow<'a, [u8]> {
    fn from(c: CowBytes<'a>) -> Self {
        c.0
    }
}

impl<'a> Deref for CowBytes<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
