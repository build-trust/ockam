#[cfg(feature = "heapless")]
use heapless::consts::*;

cfg_if!{
    if #[cfg(feature = "heapless")] {
        pub type ErrorStr = heapless::String<U32>;
    }
    else {
        pub type ErrorStr = String;
    }
}

/// The general error returned by Ockam functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub inner: ErrorStr,
    pub code: u32,
}


impl Error {
    /// Return the values for success
    pub fn success() -> Self { Self { inner: ErrorStr::new(), code: 0 } }

    pub fn from_msg(code: u32, msg: &str) -> Self {
        let inner = ErrorStr::from(msg);
        Self {
            inner, code
        }
    }
}

/// The result type returned by Ockam functions
pub type Result<T> = std::result::Result<T, Error>;