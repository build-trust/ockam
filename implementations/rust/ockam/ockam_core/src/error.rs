// ---
// Export std_error if standard library is present.

#[cfg(feature = "std")]
mod std_error;
#[cfg(feature = "std")]
pub use std_error::*;

// ---
// Export no_std_error if standard library is not present.

#[cfg(not(feature = "std"))]
mod no_std_error;
#[cfg(not(feature = "std"))]
pub use no_std_error::*;
