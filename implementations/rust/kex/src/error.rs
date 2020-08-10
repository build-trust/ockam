use failure::Fail;
use ockam_common::error::ErrorKind;

/// Represents the failures that can occur in
/// an Ockam Key Exchange
#[derive(Clone, Copy, Fail, Debug)]
pub enum KeyExchangeFailErrorKind {
    /// An invalid number of bytes was received in an exchange
    #[fail(display = "An invalid number of bytes was received in an exchange. Expected: {}, found: {}", 0, 0)]
    InvalidByteCount(usize, usize),
    /// An invalid parameter was supplied: {}
    #[fail(display = "An invalid parameter was supplied: {}", 0)]
    InvalidParam(usize),
}

impl ErrorKind for KeyExchangeFailErrorKind {
    const ERROR_INTERFACE: usize = 5 << 24;

    fn to_usize(&self) -> usize {
        match *self {
            KeyExchangeFailErrorKind::InvalidByteCount(..) => Self::ERROR_INTERFACE | 2,
            KeyExchangeFailErrorKind::InvalidParam(..) => Self::ERROR_INTERFACE | 3,
        }
    }
}

