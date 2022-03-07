use crate::Error;

/// A routing specific error type.
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

#[cfg(test)]
mod tests {
    use super::RouteError;
    use crate::{compat::collections::HashMap, Error};
    use core::array::IntoIter;

    #[test]
    fn code_and_domain() {
        let errors_map =
            IntoIter::new([(000, RouteError::IncompleteRoute)]).collect::<HashMap<_, _>>();

        for (expected_code, err) in errors_map {
            let err: Error = err.into();
            #[cfg(feature = "alloc")]
            assert_eq!(err.domain(), RouteError::DOMAIN_NAME);
            assert_eq!(err.code(), RouteError::DOMAIN_CODE + expected_code);
        }
    }
}
