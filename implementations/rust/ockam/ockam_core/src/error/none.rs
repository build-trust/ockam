use core::fmt::{Debug, Display};

/// A simple error type that expresses a None type
#[derive(Clone, Copy, Debug)]
pub struct NoneError;

impl Display for NoneError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Option<T> was None")
    }
}

impl std::error::Error for NoneError {}
