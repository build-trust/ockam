use core::cell::{RefCell, UnsafeCell};
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

use ockam_core::compat::sync::Arc;

use futures::future::poll_fn;
use futures::task::AtomicWaker;

use heapless::mpmc::MpMcQueue;

pub type QueueN<T, const N: usize> = MpMcQueue<T, N>;
pub type Queue<T> = QueueN<T, 32>;

pub fn channel<T>(_size: usize) -> (Sender<T>, Receiver<T>) {
    let queue = Queue::new();
    channel_with_queue(queue)
}

fn channel_with_queue<T>(queue: Queue<T>) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        queue: queue,
        wake_sender: AtomicWaker::new(),
        wake_receiver: AtomicWaker::new(),
        sender_count: AtomicUsize::new(1),
        is_sender_closed: AtomicBool::new(false),
    });
    let inner_clone = Arc::clone(&inner);
    (Sender(inner), Receiver(inner_clone))
}

struct Inner<T> {
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

pub struct Sender<T>(Arc<Inner<T>>);

impl<T: core::fmt::Debug> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> {
        let mut value = Some(value);
        poll_fn(|context| {
            match self.0.queue.enqueue(value.take().unwrap()) {
                Ok(()) => {
                    self.0.wake_receiver.wake();
                    Poll::Ready(Ok(()))
                }
                Err(_) => {
                    // queue is full
                    {
                        self.0.is_sender_closed.swap(true, Ordering::AcqRel);
                        self.0.wake_receiver.wake();
                    }
                    self.0.wake_sender.register(&context.waker());
                    Poll::Pending
                }
            }
        })
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

pub struct Receiver<T>(Arc<Inner<T>>);

impl<T: core::fmt::Debug> Receiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        let result = poll_fn(|context| match self.0.queue.dequeue() {
            Some(value) => {
                self.0.wake_sender.wake();
                Poll::Ready(Some(value))
            }
            None => {
                self.0.wake_receiver.register(&context.waker());
                if self.0.is_sender_closed.load(Ordering::Acquire) {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        })
        .await;
        result
    }
}

impl<T> core::fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "[Receiver]")
    }
}

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
