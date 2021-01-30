use ockam_core::Error;

/// Error declarations.
#[derive(Clone, Copy, Debug)]
pub enum NodeError {
    /// No error.
    None,

    /// Unable to gracefully stop the Node.
    CouldNotStop,
}

impl NodeError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 11_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_NODE";
}

impl Into<Error> for NodeError {
    fn into(self) -> Error {
        Error::new(Self::DOMAIN_CODE + (self as u32), Self::DOMAIN_NAME)
    }
}
