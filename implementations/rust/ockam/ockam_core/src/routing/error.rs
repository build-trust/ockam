use crate::{
    errcode::{Kind, Origin},
    Error,
};

/// A routing specific error type.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum RouteError {
    /// Message had an incomplete route
    #[error("incomplete route")]
    IncompleteRoute,
}
impl From<RouteError> for Error {
    fn from(err: RouteError) -> Self {
        let kind = match err {
            RouteError::IncompleteRoute => Kind::Misuse,
        };
        Error::new(Origin::Core, kind, err)
    }
}
