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
