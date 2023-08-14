#[allow(clippy::module_inception)]
mod storage;

mod memory;

/// LMDB implementation of the Storage trait
#[cfg(feature = "std")]
pub mod lmdb_storage;
/// Sqlite implementation of the Storage trait
#[cfg(feature = "sqlite")]
pub mod sqlite_storage;

pub use memory::*;
pub use storage::*;

#[cfg(feature = "std")]
pub use lmdb_storage::*;

#[cfg(feature = "sqlite")]
pub use sqlite_storage::*;
