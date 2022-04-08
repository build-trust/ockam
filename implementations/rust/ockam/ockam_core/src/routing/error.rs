use crate::{
    errcode::{Kind, Origin},
    Error,
};

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
impl core::fmt::Display for RouteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RouteError::IncompleteRoute => "incomplete route".fmt(f),
        }
    }
}
