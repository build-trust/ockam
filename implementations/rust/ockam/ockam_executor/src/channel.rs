use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

use futures::task::AtomicWaker;
use futures::FutureExt;
use heapless::mpmc::MpMcQueue;

use ockam_core::compat::sync::Arc;

pub type QueueN<T, const N: usize> = MpMcQueue<T, N>;
pub type Queue<T> = QueueN<T, QUEUE_LENGTH>;

pub fn channel<T>(length: usize) -> (Sender<T>, Receiver<T>) {
    let queue = Queue::new();
    channel_with_queue(length, queue)
}

fn channel_with_queue<T>(length: usize, queue: Queue<T>) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        _length: length,
        queue: queue,
        item_count: AtomicUsize::new(0),
        wake_sender: AtomicWaker::new(),
        wake_receiver: AtomicWaker::new(),
        sender_count: AtomicUsize::new(1),
        is_sender_closed: AtomicBool::new(false),
    });
    let inner_clone = Arc::clone(&inner);
    (Sender(inner), Receiver(inner_clone))
}

/// Inner
struct Inner<T> {
    /// Logical length of the underlying queue
    _length: usize,

    /// Number of items in queue
    item_count: AtomicUsize,

    /// Shared instance of the underlying queue
    queue: Queue<T>,

    /// Notifies all tasks listening for the receiver being dropped
    wake_sender: AtomicWaker,

    /// Receiver waker. Notified when a value is pushed into the channel.
    wake_receiver: AtomicWaker,

    /// Tracks the number of outstanding sender handles.
    ///
    /// When this drops to zero, the send half of the channel is closed.
    sender_count: AtomicUsize,

    /// Set to true when the send half of the channel is closed.
    is_sender_closed: AtomicBool,
}

impl<T> Inner<T> {
    fn _len(&self) -> usize {
        self._length
    }
}

/// Sender
pub struct Sender<T>(Arc<Inner<T>>);

impl<T: core::fmt::Debug> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> {
        SendFuture {
            inner: &self.0,
            value: Some(value),
        }
        .await
    }

    pub async fn closed(&self) {
        unimplemented!();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.0.sender_count.fetch_add(1, Ordering::Relaxed);
        Sender(self.0.clone())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let sender_count = self.0.sender_count.fetch_sub(1, Ordering::AcqRel);
        if sender_count != 1 {
            return;
        }

        // close the list
        self.0.is_sender_closed.swap(true, Ordering::AcqRel);

        // notify the receiver
        self.0.wake_receiver.wake();
    }
}

impl<T> core::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Sender")
    }
}

pub struct SendFuture<'a, T> {
    inner: &'a Inner<T>,
    value: Option<T>,
}

impl<'a, T> Future for SendFuture<'a, T>
where
    T: core::fmt::Debug,
{
    type Output = Result<(), error::SendError<T>>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let value = self.value.take();
        match value {
            Some(value) => {
                match self.inner.queue.enqueue(value) {
                    Ok(()) => {
                        self.inner.item_count.fetch_add(1, Ordering::Relaxed);
                        self.inner.wake_receiver.wake();
                        Poll::Ready(Ok(()))
                    }
                    Err(_) => {
                        // queue is full - TODO implement backpressure
                        {
                            error!("[channel] queue overflowed");
                            self.inner.is_sender_closed.swap(true, Ordering::AcqRel);
                            self.inner.wake_receiver.wake();
                        }
                        self.inner.wake_sender.register(context.waker());
                        Poll::Pending
                    }
                }
            }
            None => panic!("[channel] Value cannot be None"),
        }
    }
}

impl<'a, T> Unpin for SendFuture<'a, T> {}

/// Receiver
pub struct Receiver<T>(Arc<Inner<T>>);

impl<T: core::fmt::Debug> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        ReceiveFuture { inner: &self.0 }.await
    }

    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        ReceiveFuture { inner: &self.0 }.poll_unpin(cx)
    }
}

impl<T> core::fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "[Receiver]")
    }
}

pub struct ReceiveFuture<'a, T> {
    inner: &'a Inner<T>,
}

impl<'a, T> Future for ReceiveFuture<'a, T>
where
    T: core::fmt::Debug,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.inner.queue.dequeue() {
            Some(value) => {
                self.inner.item_count.fetch_sub(1, Ordering::Relaxed);
                self.inner.wake_sender.wake();
                Poll::Ready(Some(value))
            }
            None => {
                self.inner.wake_receiver.register(context.waker());
                if self.inner.is_sender_closed.load(Ordering::Acquire) {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

/// channel::error
pub mod error {
    use core::fmt;

    #[derive(Debug)]
    pub struct SendError<T>(pub T);

    impl<T> fmt::Display for SendError<T> {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(fmt, "SendError -> channel closed")
        }
    }
}
