use ockam_core::{Error, Message};
use ockam_vault::AeadSecretKeyHandle;
use serde::{Deserialize, Serialize};

/// Request type for `SecureChannel` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum SecureChannelApiRequest {
    /// Derive a new key from current key and shutdown the worker
    ExtractKey,
}

/// Response type for `SecureChannel` API Address
#[derive(Serialize, Deserialize, Message)]
pub enum SecureChannelApiResponse {
    /// Success
    Ok(AeadSecretKeyHandle),
    /// Error
    Err(Error),
}
