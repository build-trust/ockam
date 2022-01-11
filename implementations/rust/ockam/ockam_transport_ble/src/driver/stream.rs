use ockam_core::compat::sync::Arc;

#[cfg(feature = "std")]
use futures::lock::Mutex;

#[cfg(not(feature = "std"))]
use super::mutex::Mutex;

use crate::driver::BleStreamDriver;

pub(crate) struct AsyncStream<A>
where
    A: BleStreamDriver + Send,
{
    inner: Arc<Mutex<A>>,
}

impl<A> Clone for AsyncStream<A>
where
    A: BleStreamDriver + Send,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<A> AsyncStream<A>
where
    A: BleStreamDriver + Send,
{
    pub(crate) fn with_ble_device(ble_device: A) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ble_device)),
        }
    }

    pub(crate) fn split(self) -> (Sink<A>, Source<A>) {
        let sink = Sink {
            inner: Arc::new(self.clone()),
        };
        let source = Source {
            inner: Arc::new(self),
        };
        (sink, source)
    }
}

impl<A> AsyncStream<A>
where
    A: BleStreamDriver + Send,
{
    async fn write(&self, buffer: &[u8]) -> ockam::Result<()> {
        let mut guard = self.inner.lock().await;
        (*guard).write(buffer).await
    }

    async fn poll<'a, 'b>(
        &'a self,
        buffer: &'b mut [u8],
    ) -> ockam::Result<crate::driver::BleEvent<'b>> {
        let mut guard = self.inner.lock().await;
        (*guard).poll(buffer).await
    }
}

/// A Sink for writing data buffers to the Ble adapter
pub(crate) struct Sink<A>
where
    A: BleStreamDriver + Send,
{
    inner: Arc<AsyncStream<A>>,
}

impl<A> Sink<A>
where
    A: BleStreamDriver + Send,
{
    pub async fn write(&self, buffer: &[u8]) -> ockam::Result<()> {
        self.inner.write(buffer).await
    }
}

/// A Source for reading data buffers from the Ble adapter
pub(crate) struct Source<A>
where
    A: BleStreamDriver + Send,
{
    inner: Arc<AsyncStream<A>>,
}

impl<A> Source<A>
where
    A: BleStreamDriver + Send,
{
    pub async fn poll<'a, 'b>(
        &'a self,
        buffer: &'b mut [u8],
    ) -> ockam::Result<crate::driver::BleEvent<'b>> {
        self.inner.poll(buffer).await
    }
}
