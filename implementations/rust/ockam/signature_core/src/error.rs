use crate::lib::*;

/// An error that can be thrown from short group signatures
#[derive(Debug)]
pub struct Error {
    code: u32,
    message: String<64>,
}

impl Error {
    #[cfg(not(feature = "std"))]
    /// Create a new error
    pub fn new(code: u32, message: &str) -> Self {
        Self {
            code,
            message: String::from(message),
        }
    }

    /// Return the error code
    #[inline]
    pub fn code(&self) -> u32 {
        self.code
    }

    /// Return the error message
    #[inline]
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error {{ code: {}, message: \"{}\" }}",
            self.code, self.message
        )
    }
}
