use core::fmt::{self, Debug, Display};
use minicbor::{CborLen, Decode, Encode};
use serde::{Deserialize, Serialize};

/// The transport type of an [`Address`](crate::Address).
#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    CborLen,
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct TransportType(#[n(0)] u8);

/// The local transport type.
pub const LOCAL: TransportType = TransportType::new(0);

impl TransportType {
    /// Create a new transport type.
    pub const fn new(n: u8) -> Self {
        TransportType(n)
    }

    /// Raw value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Is this the local transport type?
    pub fn is_local(self) -> bool {
        self == LOCAL
    }
}

impl Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<TransportType> for u8 {
    fn from(ty: TransportType) -> Self {
        ty.0
    }
}
