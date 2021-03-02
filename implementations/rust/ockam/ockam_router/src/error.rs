use ockam_core::Error;

/// Represents the failures that can occur in
/// Ockam Connection and Listener traits
#[derive(Clone, Copy, Debug)]
pub enum RouterError {
    None,
    Stop,
    NoRoute,
    TypeIdInUse,
    NoSuchType,
    KeyInUse,
    NoSuchKey,
}

impl RouterError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 16_000;
    /// Error domain
    pub const DOMAIN_NAME: &'static str = "OCKAM_ROUTER";
}

impl Into<Error> for RouterError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
