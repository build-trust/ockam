#[macro_use]
extern crate alloc;

pub use ockam_error::*;
// re-export the #[node] attribute macro.
pub use ockam_node_attribute::*;

#[derive(Clone, Copy, Debug)]
pub enum Error {
    None,
    WorkerRuntime,
}

impl Error {
    /// Error domain
    pub const ERROR_DOMAIN: &'static str = "OCKAM_ERROR_DOMAIN";
}

impl Into<OckamError> for Error {
    fn into(self) -> OckamError {
        OckamError::new(self as u32, Error::ERROR_DOMAIN)
    }
}

pub mod entity;
