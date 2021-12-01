#![allow(missing_docs)]
#![allow(clippy::needless_lifetimes)]

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use ockam_core::compat::io;
use ockam_core::compat::sync::{Arc, Mutex};

use crate::executor;

/// execute
pub fn execute<'r, F>(_runtime: &'r Runtime, future: F) -> <F as Future>::Output
where
    F: Future<Output = ()> + Send,
    F::Output: Send,
{
    executor::current().block_on(future)
}

/// block_future
pub fn block_future<'r, F>(_runtime: &'r Runtime, _future: F) -> <F as Future>::Output
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

/// spawn
pub fn spawn<F: 'static>(_future: F)
where
    F: Future + Send,
    F::Output: Send,
{
    // task::spawn(f)
    unimplemented!();
}

/// Runtime
pub struct Runtime {}

impl Runtime {
    pub fn new() -> io::Result<Runtime> {
        Ok(Self {})
    }

    /// Spawn a future onto the runtime.
    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        executor::current().spawn(future);
        JoinHandle::new()
    }
}

/// SharedJoinHandle
pub struct SharedJoinHandle<T> {
    pub value: Option<T>,
    pub waker: Option<Waker>,
}

/// JoinHandle
pub struct JoinHandle<T>(pub Arc<Mutex<SharedJoinHandle<T>>>);

impl<T: Send> Default for SharedJoinHandle<T> {
    fn default() -> SharedJoinHandle<T> {
        Self {
            value: None,
            waker: None,
        }
    }
}

impl<T: Send> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut guard = self.0.lock().unwrap();
        if let Some(value) = guard.value.take() {
            return Poll::Ready(value);
        }
        guard.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<T: Send> JoinHandle<T> {
    pub fn new() -> JoinHandle<T> {
        let inner = Arc::new(Mutex::new(SharedJoinHandle::default()));
        JoinHandle(inner)
    }
}

impl<T: Send> Default for JoinHandle<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// yield_now
pub async fn yield_now() {
    #[allow(dead_code)]
    struct YieldNow {
        yielded: bool,
    }

    impl Future for YieldNow {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                Poll::Ready(())
            } else {
                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    YieldNow { yielded: false }.await
}
