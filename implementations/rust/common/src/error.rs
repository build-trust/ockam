use std::fmt::{Display, Formatter};

/// Error king
pub trait ErrorKind {
    /// Error interface bit
    const ERROR_INTERFACE: usize;
    /// to usize
    fn to_usize(&self) -> usize;
}

/// Common error
#[derive(Debug)]
pub struct OckamError {
    code: u32,
    domain: &'static str,
}

impl Display for OckamError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error code: {}, domain: {}", self.code, self.domain)
    }
}

impl OckamError {
    /// Error code
    pub fn code(&self) -> u32 {
        self.code
    }
    /// Error domain
    pub fn domain(&self) -> &str {
        &self.domain
    }
}

impl OckamError {
    /// Create new error
    pub fn new(code: u32, domain: &'static str) -> Self {
        Self { code, domain }
    }
}

cfg_if! {
    if #[cfg(feature = "ffi")] {
        use ffi_support::{ErrorCode, ExternError};

        impl From<OckamError> for ExternError {
            fn from(err: OckamError) -> Self {
                ExternError::new_error(ErrorCode::new(err.code as i32), err.domain)
            }
        }
    }
}

/// OckamResult
pub type OckamResult<T> = Result<T, OckamError>;
