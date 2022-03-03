use ockam_core::Error;

/// Types of errors that may occur constructing a secure channel.
#[derive(Debug, Clone, Copy)]
pub enum SecureChannelError {
    /// The key exchange process failed.
    KeyExchange = 1,
    /// Internal state is invalid.
    InvalidInternalState,
    /// Expected nonce was invalid.
    InvalidNonce,
    /// Key exchange process did not complete.
    KeyExchangeNotComplete,
    /// Invalid response received from the Hub.
    InvalidHubResponse,
    /// Invalid LocalInfo type
    InvalidLocalInfoType,
}

impl SecureChannelError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 16_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_SECURE_CHANNEL";
}

impl From<SecureChannelError> for Error {
    fn from(e: SecureChannelError) -> Error {
        Error::new(
            SecureChannelError::DOMAIN_CODE + (e as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), e),
        )
    }
}
