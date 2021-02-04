#[derive(Clone, Copy, Debug)]
pub enum Error {
    None,
}

impl Error {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 10_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_NODE";
}

impl From<Error> for ockam_core::Error {
    fn from(e: Error) -> ockam_core::Error {
        ockam_core::Error::new(Error::DOMAIN_CODE + (e as u32), Error::DOMAIN_NAME)
    }
}
