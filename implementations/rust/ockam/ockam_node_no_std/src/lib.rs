#![cfg_attr(not(feature = "std"), no_std)]
//! `no_std` implementation of Ockam Node

use core::future::Future;

/// Block on the execution of the given future.
pub fn block_on<T>(future: impl Future<Output = T> + 'static + Send) -> T
where
    T: Send + 'static,
{
    executor::block_on(future)
}
