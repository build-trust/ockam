use minicbor::{CborLen, Decode, Encode};
use ockam_vault::AeadSecretKeyHandle;

/// Request type for `SecureChannel` API Address
#[derive(Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum SecureChannelApiRequest {
    /// Derive a new key from current key and shutdown the worker
    #[n(0)] ExtractKey,
}

/// Response type for `SecureChannel` API Address
#[derive(Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum SecureChannelApiResponse {
    /// Success
    #[n(0)] Ok(#[n(0)] AeadSecretKeyHandle),
    /// Error
    #[n(1)] Err(#[n(0)] String),
}
