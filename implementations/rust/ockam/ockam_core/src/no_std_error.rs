use core::fmt::{Debug, Display};

/// Implementation of std::error::Error for no_std platforms.
pub trait Error: Debug + Display {
    /// The underlying cause of this error, if any.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
