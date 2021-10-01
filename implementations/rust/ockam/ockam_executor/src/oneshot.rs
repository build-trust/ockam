use core::cell::{RefCell, UnsafeCell};
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::{Context, Poll};

use ockam_core::compat::sync::Arc;

use futures::future::poll_fn;
use futures::task::AtomicWaker;

use heapless::mpmc::MpMcQueue;

type QueueN<T, const N: usize> = MpMcQueue<T, N>;
pub type Queue<T> = QueueN<T, 32>;

pub type Sender<T> = SyncSender<T>;

pub fn sync_channel<T>(_size: usize) -> (SyncSender<T>, Receiver<T>) {
    let queue = Queue::<T>::new();
    channel_with_queue(queue)
}

pub fn channel<T>() -> (SyncSender<T>, Receiver<T>) {
    let queue = Queue::<T>::new();
    channel_with_queue(queue)
}

fn channel_with_queue<T>(queue: Queue<T>) -> (SyncSender<T>, Receiver<T>) {
    let sender = Arc::new(Inner {
        queue: queue,
        wake_sender: AtomicWaker::new(),
        wake_receiver: AtomicWaker::new(),
        sender_count: AtomicUsize::new(1),
        is_sender_closed: AtomicBool::new(false),
    });
    let receiver = Arc::clone(&sender);
    (SyncSender(sender), Receiver(receiver))
}

struct Inner<T> {
    /// Shared instance of the underlying queue
    queue: Queue<T>,

    /// Notifies all tasks listening for the receiver being dropped
    wake_sender: AtomicWaker,

    /// Receiver waker. Notified when a value is pushed into the channel.
    wake_receiver: AtomicWaker,

    /// When this drops to zero, the send half of the channel is closed.
    sender_count: AtomicUsize,

    /// Set to true when the send half of the channel is closed.
    is_sender_closed: AtomicBool,
}

pub struct SyncSender<T>(Arc<Inner<T>>);

impl<T: core::fmt::Debug> SyncSender<T> {
    pub fn send(&self, value: T) -> Result<(), error::SendError<T>> {
        let mut value = Some(value);
        match self.0.queue.enqueue(value.take().unwrap()) {
            Ok(()) => {
                self.0.wake_receiver.wake();
                Ok(())
            }
            Err(value) => {
                // queue is full
                Err(error::SendError(value))
            }
        }
    }

    pub fn closed(&self) {
        unimplemented!();
    }
}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> Self {
        self.0.sender_count.fetch_add(1, Ordering::Relaxed);
        SyncSender(self.0.clone())
    }
}

impl<T> Drop for SyncSender<T> {
    fn drop(&mut self) {
        let sender_count = self.0.sender_count.fetch_sub(1, Ordering::AcqRel);
        if sender_count != 1 {
            return;
        }

        self.0.wake_receiver.wake();

        // close the list
        self.0.is_sender_closed.swap(true, Ordering::AcqRel);
    }
}

impl<T> core::fmt::Debug for SyncSender<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "[SyncSender]")
    }
}

// - sync_channel::Receiver ---------------------------------------------------

pub struct Receiver<T>(Arc<Inner<T>>);

impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, error::ReceiveError> {
        match self.0.queue.dequeue() {
            Some(value) => {
                self.0.wake_sender.wake();
                Ok(value)
            }
            None => Err(error::ReceiveError),
        }
    }
}

impl<T> core::fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "[Receiver]")
    }
}

impl<T> Future for Receiver<T> {
    type Output = Result<T, error::ReceiveError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.queue.dequeue() {
            Some(value) => {
                self.0.wake_sender.wake();
                Poll::Ready(Ok(value))
            }
            None => {
                self.0.wake_receiver.register(&context.waker());
                if self.0.is_sender_closed.load(Ordering::Acquire) {
                    panic!("called after complete");
                } else {
                    Poll::Pending
                }
            }
        }
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

    #[derive(Debug)]
    pub struct ReceiveError;

    impl fmt::Display for ReceiveError {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(fmt, "ReceiveError -> all send channels are closed")
        }
    }
}
