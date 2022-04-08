use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Types of errors that may occur constructing a secure channel.
#[derive(Clone, Debug)]
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

#[allow(clippy::from_over_into)]
impl Into<Error> for SecureChannelError {
    fn into(self) -> Error {
        use SecureChannelError::*;
        let kind = match self {
            KeyExchange | KeyExchangeNotComplete => Kind::Protocol,
            InvalidInternalState | InvalidNonce | InvalidHubResponse | InvalidLocalInfoType => {
                Kind::Invalid
            }
        };

        Error::new(Origin::Channel, kind, self)
    }
}

impl ockam_core::compat::error::Error for SecureChannelError {}
impl core::fmt::Display for SecureChannelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::KeyExchange => "the key exchange process failed.".fmt(f),
            Self::InvalidInternalState => "internal state is invalid.".fmt(f),
            Self::InvalidNonce => "expected nonce was invalid.".fmt(f),
            Self::KeyExchangeNotComplete => "key exchange process did not complete.".fmt(f),
            Self::InvalidHubResponse => "invalid response received from the Hub.".fmt(f),
            Self::InvalidLocalInfoType => "invalid LocalInfo type".fmt(f),
        }
    }
}
