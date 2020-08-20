use failure::{Backtrace, Context, Fail};
use ockam_kex::error::KeyExchangeFailErrorKind;
use std::fmt;

/// Represents the failures that can occur in
/// an Ockam Channel
#[derive(Clone, Copy, Fail, Debug)]
pub enum ChannelErrorKind {
    /// An invalid parameter was supplied
    #[fail(display = "An invalid parameter was supplied: {}", 0)]
    InvalidParam(usize),
    /// An unimplemented feature was requested
    #[fail(display = "The requested feature is not supported")]
    NotImplemented,
    /// An error occurred while executing the key agreement
    #[fail(display = "An error occurred while executing the key agreement: {}", 0)]
    KeyAgreement(KeyExchangeFailErrorKind),
    /// An error occurred with the internal state of the channel
    #[fail(display = "An error occurred with the internal state of the channel")]
    State
}

impl ChannelErrorKind {
    pub(crate) const ERROR_INTERFACE_CHANNEL: usize = 8 << 24;
    /// Convert to an integer
    pub fn to_usize(&self) -> usize {
        match *self {
            ChannelErrorKind::InvalidParam(_) => Self::ERROR_INTERFACE_CHANNEL | 1,
            ChannelErrorKind::NotImplemented => Self::ERROR_INTERFACE_CHANNEL | 2,
            ChannelErrorKind::KeyAgreement(_) => Self::ERROR_INTERFACE_CHANNEL | 3,
            ChannelErrorKind::State => Self::ERROR_INTERFACE_CHANNEL | 4,
        }
    }
}

/// Wraps an error kind with context and backtrace logic
#[derive(Debug)]
pub struct ChannelError {
    inner: Context<ChannelErrorKind>,
}

impl ChannelError {
    /// Convert from an error kind and a static string
    pub fn from_msg<D>(kind: ChannelErrorKind, msg: D) -> Self
        where
            D: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        Self {
            inner: Context::new(msg).context(kind),
        }
    }

    /// Convert to an integer, reused in From trait implementations
    fn to_usize(&self) -> usize {
        self.inner.get_context().to_usize()
    }
}

impl From<ChannelErrorKind> for ChannelError {
    fn from(kind: ChannelErrorKind) -> Self {
        Self {
            inner: Context::new("").context(kind),
        }
    }
}

impl From<ChannelError> for ChannelErrorKind {
    fn from(err: ChannelError) -> Self {
        *err.inner.get_context()
    }
}

from_int_impl!(ChannelError, u32);
from_int_impl!(ChannelError, u64);
from_int_impl!(ChannelError, u128);
from_int_impl!(ChannelErrorKind, u32);
from_int_impl!(ChannelErrorKind, u64);
from_int_impl!(ChannelErrorKind, u128);

impl Fail for ChannelError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;

        for cause in Fail::iter_chain(&self.inner) {
            if first {
                first = false;
                writeln!(f, "Error: {}", cause)?;
            } else {
                writeln!(f, "Caused by: {}", cause)?;
            }
        }
        Ok(())
    }
}