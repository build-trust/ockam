use crate::{
    errcode::{Kind, Origin},
    Error,
};
use core::fmt::{self, Debug, Display};

/// A routing specific error type.
#[derive(Clone, Copy, Debug)]
pub enum RouteError {
    /// Message had an incomplete route
    IncompleteRoute,
}

impl From<RouteError> for Error {
    #[track_caller]
    fn from(err: RouteError) -> Self {
        let kind = match err {
            RouteError::IncompleteRoute => Kind::Misuse,
        };
        Error::new(Origin::Core, kind, err)
    }
}

impl crate::compat::error::Error for RouteError {}
impl Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteError::IncompleteRoute => write!(f, "incomplete route"),
        }
    }
}

/// An error which is returned when address parsing from string fails.
#[derive(Debug)]
pub struct AddressParseError {
    kind: AddressParseErrorKind,
}

/// Enum to store the cause of an address parsing failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AddressParseErrorKind {
    /// Unable to parse address num in the address string.
    InvalidType(core::num::ParseIntError),
    /// Address string has more than one '#' separator.
    MultipleSep,
}

impl AddressParseError {
    /// Create new address parse error instance.
    pub fn new(kind: AddressParseErrorKind) -> Self {
        Self { kind }
    }
    /// Return the cause of the address parsing failure.
    pub fn kind(&self) -> &AddressParseErrorKind {
        &self.kind
    }
}

impl Display for AddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            AddressParseErrorKind::InvalidType(e) => {
                write!(f, "Failed to parse address type: '{}'", e)
            }
            AddressParseErrorKind::MultipleSep => {
                write!(
                    f,
                    "Invalid address string: more than one '#' separator found"
                )
            }
        }
    }
}

impl crate::compat::error::Error for AddressParseError {}
