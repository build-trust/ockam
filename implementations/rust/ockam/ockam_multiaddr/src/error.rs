use super::Code;
use alloc::string::{String, ToString};
use core::fmt;
use ockam_core::errcode::{Kind, Origin};

#[derive(Debug)]
pub struct Error(ErrorImpl);

impl Error {
    pub fn message<D: fmt::Display>(d: D) -> Self {
        Error(ErrorImpl::Message(d.to_string()))
    }

    #[cfg(feature = "std")]
    pub fn custom(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error(ErrorImpl::Custom(e))
    }

    pub fn required_bytes(c: Code, n: usize) -> Self {
        Error(ErrorImpl::RequiredBytes(c, n))
    }

    pub fn unregistered(c: Code) -> Self {
        Error(ErrorImpl::Unregistered(c))
    }

    pub fn unregistered_prefix<S: Into<String>>(s: S) -> Self {
        Error(ErrorImpl::UnregisteredPrefix(s.into()))
    }

    pub(crate) fn invalid_proto(c: Code) -> Self {
        Error(ErrorImpl::InvalidProto(c))
    }

    pub(crate) fn into_impl(self) -> ErrorImpl {
        self.0
    }
}

#[derive(Debug)]
pub(crate) enum ErrorImpl {
    Unregistered(Code),
    InvalidProto(Code),
    UnregisteredPrefix(String),
    InvalidVarint(unsigned_varint::decode::Error),
    Message(String),
    Format(fmt::Error),
    RequiredBytes(Code, usize),
    #[cfg(feature = "std")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Unregistered(c) => write!(f, "unregistered protocol (code {c})"),
            ErrorImpl::InvalidProto(c) => write!(f, "invalid protocol value (code {c})"),
            ErrorImpl::UnregisteredPrefix(s) => write!(f, "unregistered protocol prefix {s:?}"),
            ErrorImpl::Message(m) => write!(f, "{m}"),
            ErrorImpl::InvalidVarint(e) => e.fmt(f),
            ErrorImpl::Format(e) => e.fmt(f),
            ErrorImpl::RequiredBytes(c, n) => write!(f, "value of protocol {c} requires {n} bytes"),
            #[cfg(feature = "std")]
            ErrorImpl::Custom(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::InvalidVarint(e) => Some(e),
            ErrorImpl::Custom(e) => Some(&**e),
            ErrorImpl::Format(e) => Some(e),
            ErrorImpl::InvalidProto(_)
            | ErrorImpl::RequiredBytes(..)
            | ErrorImpl::Unregistered(_)
            | ErrorImpl::UnregisteredPrefix(_)
            | ErrorImpl::Message(_) => None,
        }
    }
}

impl From<unsigned_varint::decode::Error> for Error {
    fn from(e: unsigned_varint::decode::Error) -> Self {
        Error(ErrorImpl::InvalidVarint(e))
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Error(ErrorImpl::Format(e))
    }
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> Self {
        ockam_core::Error::new(Origin::Unknown, Kind::Invalid, e)
    }
}
