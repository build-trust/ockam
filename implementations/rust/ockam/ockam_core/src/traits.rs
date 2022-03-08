//! The traits module provides extended implementations of standard traits.

use crate::compat::boxed::Box;
use crate::error::Result;

/// Clone trait for async structs.
#[async_trait]
pub trait AsyncTryClone: Sized {
    /// Try cloning a object and return an `Err` in case of failure.
    async fn async_try_clone(&self) -> Result<Self>;
}

#[async_trait]
impl<D> AsyncTryClone for D
where
    D: Clone + Sync,
{
    async fn async_try_clone(&self) -> Result<Self> {
        Ok(self.clone())
    }
}
