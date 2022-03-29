use crate::error::{
    code::{ErrorCode, Kind, Origin},
    Error2,
};

/// A routing specific error type.
#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum RouteError {
    /// Message had an incomplete route
    #[error("incomplete route")]
    IncompleteRoute,
}
impl From<RouteError> for Error2 {
    fn from(err: RouteError) -> Self {
        let kind = match err {
            RouteError::IncompleteRoute => Kind::Misuse,
        };
        Error2::new(ErrorCode::new(Origin::Core, kind), err)
    }
}
