use failure::{Context, Fail};
use ockam_common::error::ErrorKind;
use ockam_vault::error::{VaultFailError, VaultFailErrorKind};

/// Represents the failures that can occur in
/// an Ockam Key Exchange
#[derive(Clone, Fail, Debug)]
pub enum KeyExchangeFailErrorKind {
    /// An invalid number of bytes was received in an exchange
    #[fail(
        display = "An invalid number of bytes was received in an exchange. Expected: {}, found: {}",
        0, 1
    )]
    InvalidByteCount(usize, usize),
    /// An invalid parameter was supplied: {}
    #[fail(display = "An invalid parameter was supplied: {}", 0)]
    InvalidParam(usize),
    /// Happens when the Key exchange method is called out of sequence
    #[fail(
        display = "{} called out of sequence. Expected {} to be called",
        actual, expected
    )]
    MethodCalledOutOfSequence {
        /// What was received
        actual: &'static str,
        /// What was expected
        expected: &'static str,
    },
    /// Happens when a hash value is expected but finds another
    #[fail(display = "Expected hash {}, found {} ", expected, actual)]
    InvalidHash {
        /// What was expected
        expected: String,
        /// What was received
        actual: String,
    },
}

impl ErrorKind for KeyExchangeFailErrorKind {
    const ERROR_INTERFACE: usize = 5 << 24;

    fn to_usize(&self) -> usize {
        match *self {
            KeyExchangeFailErrorKind::InvalidByteCount(..) => Self::ERROR_INTERFACE | 2,
            KeyExchangeFailErrorKind::InvalidParam(..) => Self::ERROR_INTERFACE | 3,
            KeyExchangeFailErrorKind::MethodCalledOutOfSequence { .. } => Self::ERROR_INTERFACE | 4,
            KeyExchangeFailErrorKind::InvalidHash { .. } => Self::ERROR_INTERFACE | 5,
        }
    }
}

impl From<KeyExchangeFailErrorKind> for KexExchangeFailError {
    fn from(err: KeyExchangeFailErrorKind) -> Self {
        Self {
            inner: Context::new("").context(err),
        }
    }
}

impl From<KexExchangeFailError> for KeyExchangeFailErrorKind {
    fn from(err: KexExchangeFailError) -> Self {
        err.inner.get_context().clone()
    }
}

impl From<VaultFailError> for KexExchangeFailError {
    fn from(err: VaultFailError) -> Self {
        let err: VaultFailErrorKind = err.into();
        match err {
            VaultFailErrorKind::InvalidParam(p) => KeyExchangeFailErrorKind::InvalidParam(p).into(),
            _ => KeyExchangeFailErrorKind::MethodCalledOutOfSequence {
                actual: "",
                expected: "",
            }
            .into(),
        }
    }
}

impl From<KexExchangeFailError> for VaultFailError {
    fn from(err: KexExchangeFailError) -> Self {
        let err = err.inner.get_context();
        match err {
            KeyExchangeFailErrorKind::InvalidParam(p) => {
                VaultFailErrorKind::InvalidParam(*p).into()
            }
            KeyExchangeFailErrorKind::InvalidByteCount(_, _) => {
                VaultFailErrorKind::InvalidSize.into()
            }
            KeyExchangeFailErrorKind::MethodCalledOutOfSequence { .. } => {
                VaultFailErrorKind::InvalidContext.into()
            }
            KeyExchangeFailErrorKind::InvalidHash { .. } => VaultFailErrorKind::Ecdh.into(),
        }
    }
}

#[cfg(feature = "ffi")]
impl From<KeyExchangeFailErrorKind> for ffi_support::ExternError {
    fn from(err: KeyExchangeFailErrorKind) -> ffi_support::ExternError {
        ffi_support::ExternError::new_error(ffi_support::ErrorCode::new(err.to_usize() as i32), "")
    }
}

/// Wraps an error kind with context and backtrace logic
#[derive(Debug)]
pub struct KexExchangeFailError {
    inner: Context<KeyExchangeFailErrorKind>,
}

impl KexExchangeFailError {
    /// Convert from an error kind and a static string
    pub fn from_msg<D>(kind: KeyExchangeFailErrorKind, msg: D) -> Self
    where
        D: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        Self {
            inner: Context::new(msg).context(kind),
        }
    }

    /// Convert to an integer, reused in From trait implementations
    pub fn to_usize(&self) -> usize {
        self.inner.get_context().to_usize()
    }
}
