mod attributes_entry;
mod identities_repository;
/// LMDB implementation of the Storage trait
#[cfg(feature = "std")]
pub mod lmdb_storage;
/// Sqlite implementation of the Storage trait
#[cfg(feature = "sqlite")]
pub mod sqlite_storage;

#[allow(clippy::module_inception)]
mod storage;

pub use attributes_entry::*;
pub use identities_repository::*;

#[cfg(feature = "std")]
pub use lmdb_storage::*;
pub use storage::*;

#[cfg(feature = "sqlite")]
pub use sqlite_storage::*;
