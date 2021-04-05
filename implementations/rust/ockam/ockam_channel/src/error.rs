use ockam_core::Error;

pub enum SecureChannelError {
    None,
    KeyExchange,
    InvalidInternalState,
    InvalidNonce,
    KeyExchangeNotComplete,
    InvalidHubResponse,
}

impl SecureChannelError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 16_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_SECURE_CHANNEL";
}

impl Into<Error> for SecureChannelError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
