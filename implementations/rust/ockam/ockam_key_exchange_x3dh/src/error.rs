use ockam_core::Error;

/// Represents the failures that can occur in
/// an Ockam X3DH kex
#[derive(Clone, Copy, Debug)]
pub enum X3DHError {
    InvalidState = 1,
    MessageLenMismatch,
    SignatureLenMismatch,
    InvalidHash,
}

impl X3DHError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 18_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_KEX_X3DH";
}

impl From<X3DHError> for Error {
    fn from(err: X3DHError) -> Self {
        Self::new(
            X3DHError::DOMAIN_CODE + (err as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), err),
        )
    }
}
