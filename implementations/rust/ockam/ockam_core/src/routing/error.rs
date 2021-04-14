use crate::Error;

/// A routing specific error type
#[derive(Clone, Copy, Debug)]
pub enum RouteError {
    /// Message had an incomplete route
    IncompleteRoute,
}

impl RouteError {
    /// Route error specific domain code
    pub const DOMAIN_CODE: u32 = 19_000;
    /// Route error specific domain name
    pub const DOMAIN_NAME: &'static str = "OCKAM_ROUTE";
}

impl From<RouteError> for Error {
    fn from(e: RouteError) -> Error {
        Error::new(
            RouteError::DOMAIN_CODE + (e as u32),
            RouteError::DOMAIN_NAME,
        )
    }
}
