#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate downcast;

#[cfg(feature = "heapless")]
pub use heapless;

pub mod vault;
pub mod error;