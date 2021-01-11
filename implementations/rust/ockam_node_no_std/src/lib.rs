#![no_std]

use core::future::Future;

pub fn block_on<T>(future: impl Future<Output = T> + 'static + Send) -> T
where
    T: Send + 'static,
{
    executor::block_on(future)
}
