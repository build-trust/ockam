#[cfg(feature = "std")]
pub mod lmdb_storage;

#[cfg(feature = "std")]
pub use lmdb_storage::*;

#[cfg(feature = "sqlite")]
pub mod sqlite_storage;

#[cfg(feature = "sqlite")]
pub use sqlite_storage::*;

use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

use crate::Expr;

/// Policy storage entry.
///
/// Used instead of storing plain `Expr` values to allow for additional
/// metadata, versioning, etc.
#[derive(Debug, Encode, Decode)]
#[rustfmt::skip]
struct PolicyEntry<'a> {
    #[b(0)] expr: Cow<'a, Expr>,
}
