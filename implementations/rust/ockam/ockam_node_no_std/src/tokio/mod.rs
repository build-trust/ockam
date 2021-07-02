// TODO document modules once we've stabilized async execution
#![allow(missing_docs)]
#![allow(clippy::needless_lifetimes)]

use async_embedded as async_cortex_m;
use core::future::Future;

pub mod sync;
pub mod time;

/// execute
pub fn execute<'r, F>(_runtime: &'r runtime::Runtime, future: F) -> <F as Future>::Output
where
    F: Future<Output = ()> + Send,
    F::Output: Send,
{
    async_cortex_m::task::block_on(future)
}

/// block_future
pub fn block_future<'r, F>(_runtime: &'r runtime::Runtime, _future: F) -> <F as Future>::Output
where
    F: Future + Send,
    F::Output: Send,
{
    // task::block_in_place(move || {
    //     let local = task::LocalSet::new();
    //     local.block_on(rt, f)
    // })
    unimplemented!();
}

// - runtime ------------------------------------------------------------------

/// runtime
pub mod runtime {
    use crate::tokio::task::JoinHandle;
    use async_embedded as async_cortex_m;
    use core::future::Future;
    use ockam_core::compat::io;

    pub struct Runtime {}

    impl Runtime {
        pub fn new() -> io::Result<Runtime> {
            Ok(Self {})
        }

        pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            async_cortex_m::task::spawn(future);
            JoinHandle::new(23)
        }
    }
}

// - task ---------------------------------------------------------------------

/// task
pub mod task {
    pub use super::block_future;

    pub type RawTask = u32;

    #[derive(Copy, Clone)]
    pub struct JoinHandle<T> {
        pub raw: Option<RawTask>,
        _p: core::marker::PhantomData<T>,
    }

    impl<T> JoinHandle<T> {
        pub(super) fn new(raw: RawTask) -> JoinHandle<T> {
            JoinHandle {
                raw: Some(raw),
                _p: core::marker::PhantomData,
            }
        }
    }
}
