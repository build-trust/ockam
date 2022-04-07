#![deny(missing_docs)]
#![allow(missing_docs)] // Contents are self describing for now.

#[derive(Clone, Copy, Debug)]
pub enum SessionManagementError {
    MismatchedRequestType = 1,
    InvalidReceiverAddress,
    NoResponderRoute,
}

impl SessionManagementError {
    /// Integer code associated with the error domain.
    pub const DOMAIN_CODE: u32 = 23_000;
    /// Descriptive name for the error domain.
    pub const DOMAIN_NAME: &'static str = "OCKAM_SESSION_MANAGEMENT";
}

impl From<SessionManagementError> for ockam_core::Error {
    fn from(e: SessionManagementError) -> ockam_core::Error {
        ockam_core::Error::new(
            SessionManagementError::DOMAIN_CODE + (e as u32),
            ockam_core::compat::format!("{}::{:?}", module_path!(), e),
        )
    }
}
