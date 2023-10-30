use ockam_core::compat::{error::Error as StdError, fmt};
use ockam_core::{
    errcode::{Kind, Origin},
    Error,
};

/// Represents the failures that can occur in
/// an Ockam XX Key Agreement
#[derive(Clone, Copy, Debug)]
pub enum XXError {
    /// An internal Vault error has occurred.
    InternalVaultError,
    /// A message had an unexpected length.
    MessageLenMismatch,
    /// Exceeded maximum allowed message length for noise
    ExceededMaxMessageLen,
    /// Invalid internal state.
    InvalidInternalState,
}

impl StdError for XXError {}

impl fmt::Display for XXError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalVaultError => write!(f, "internal vault error"),
            Self::MessageLenMismatch => write!(f, "message length mismatch"),
            Self::ExceededMaxMessageLen => {
                write!(f, "exceeded maximum allowed message length for noise")
            }
            Self::InvalidInternalState => write!(f, "invalid internal state"),
        }
    }
}

impl From<XXError> for Error {
    #[track_caller]
    fn from(err: XXError) -> Self {
        let kind = match err {
            XXError::InternalVaultError => Kind::Internal,
            XXError::MessageLenMismatch => Kind::Misuse,
            XXError::ExceededMaxMessageLen => Kind::Invalid,
            XXError::InvalidInternalState => Kind::Internal,
        };

        Error::new(Origin::KeyExchange, kind, err)
    }
}
