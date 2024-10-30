use alloc::string::String;
use core::fmt::{Display, Formatter};
use minicbor::{CborLen, Decode, Encode};

/// Identifier LEN. Should be equal to [`ockam_identity::models::IDENTIFIER_LEN`]
pub const LOCAL_INFO_IDENTIFIER_LEN: usize = 32;

/// Identity SecureChannel LocalInfo unique Identifier
pub const SECURE_CHANNEL_IDENTIFIER: &str = "SECURE_CHANNEL_IDENTIFIER";

/// Copy of [`ockam_identity::models::IDENTIFIER`]. Copied for decoupling.
#[derive(Clone, Eq, PartialEq, Hash, Encode, Decode, CborLen, Debug)]
#[cbor(transparent)]
pub struct LocalInfoIdentifier(
    #[cbor(n(0), with = "minicbor::bytes")] pub [u8; LOCAL_INFO_IDENTIFIER_LEN],
);

impl From<&LocalInfoIdentifier> for String {
    fn from(id: &LocalInfoIdentifier) -> Self {
        const PREFIX: &str = "I";
        format!("{}{}", PREFIX, hex::encode(id.0.as_ref()))
    }
}

impl From<LocalInfoIdentifier> for String {
    fn from(id: LocalInfoIdentifier) -> Self {
        String::from(&id)
    }
}

impl Display for LocalInfoIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&String::from(self))
    }
}
