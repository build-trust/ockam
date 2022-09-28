use core::fmt;

use ockam_core::compat::io;
use ockam_core::errcode::{Kind, Origin};

/// Potential API errors
#[derive(Debug)]
pub struct ApiError(ErrorImpl);

impl ApiError {
    pub fn generic(cause: &str) -> ockam_core::Error {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, cause)
    }

    pub fn message<T: fmt::Display>(m: T) -> ockam_core::Error {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, m.to_string())
    }

    pub fn wrap<E>(e: E) -> ockam_core::Error
    where
        E: ockam_core::compat::error::Error + Send + Sync + 'static,
    {
        ockam_core::Error::new(Origin::Application, Kind::Unknown, e)
    }
}

#[derive(Debug)]
enum ErrorImpl {
    CborDecode(minicbor::decode::Error),
    CborEncode(minicbor::encode::Error<io::Error>),
    SerdeJson(serde_json::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::CborEncode(e) => e.fmt(f),
            ErrorImpl::CborDecode(e) => e.fmt(f),
            ErrorImpl::SerdeJson(e) => e.fmt(f),
        }
    }
}

impl ockam_core::compat::error::Error for ApiError {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::CborDecode(e) => Some(e),
            ErrorImpl::CborEncode(e) => Some(e),
            ErrorImpl::SerdeJson(e) => Some(e),
        }
    }
}

impl From<minicbor::decode::Error> for ApiError {
    fn from(e: minicbor::decode::Error) -> Self {
        ApiError(ErrorImpl::CborDecode(e))
    }
}

impl From<minicbor::encode::Error<io::Error>> for ApiError {
    fn from(e: minicbor::encode::Error<io::Error>) -> Self {
        ApiError(ErrorImpl::CborEncode(e))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError(ErrorImpl::SerdeJson(e))
    }
}

impl From<ApiError> for ockam_core::Error {
    fn from(e: ApiError) -> Self {
        ockam_core::Error::new(Origin::Application, Kind::Invalid, e)
    }
}
