/// File implementation of a key value storage
#[cfg(feature = "std")]
mod file_key_value_storage;

/// File implementation of a value storage
#[cfg(feature = "std")]
mod file_value_storage;

/// In memory implementation of a key value storage
mod in_memory_key_value_storage;

/// In memory implementation of a value storage
mod in_memory_value_storage;

/// Trait defining the functions for a key value storage
mod key_value_storage;

/// Trait defining the functions for a value storage
mod value_storage;

#[cfg(feature = "std")]
pub use file_key_value_storage::*;
#[cfg(feature = "std")]
pub use file_value_storage::*;
pub use in_memory_key_value_storage::*;
pub use in_memory_value_storage::*;
pub use key_value_storage::*;
pub use value_storage::*;
