use ockam_common::error::OckamError;

/// Represents the failures that can occur in
/// an Ockam Kex traits
#[derive(Clone, Copy, Debug)]
pub enum Error {
    /// None
    None,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "KEX_TRAITS_ERROR_DOMAIN";
}

impl Into<OckamError> for Error {
    fn into(self) -> OckamError {
        OckamError::new(self as u32, Error::ERROR_DOMAIN)
    }
}
