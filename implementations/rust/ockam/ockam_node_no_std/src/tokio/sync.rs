/// multiple producer, single consumer async channel
pub mod mpsc {

    // make println! available for tests
    #[cfg(all(feature = "std", test))]
    #[macro_use]
    extern crate ockam_core;

    use core::sync::atomic::{
        AtomicBool, AtomicUsize,
        Ordering::{AcqRel, Relaxed},
    };
    use core::task::Poll;
    use futures::future::poll_fn;
    use futures::task::AtomicWaker;
    use heapless::mpmc::MpMcQueue;
    use ockam_core::compat::sync::Arc;

    pub type QueueN<T, const N: usize> = MpMcQueue<T, N>;
    pub type Queue<T> = QueueN<T, 32>;

    /// channel constructor
    ///
    /// TODO currently allocates a channel with a fixed size of 32
    /// irrespective of the size provided
    pub fn channel<T>(_size: usize) -> (Sender<T>, Receiver<T>) {
        let queue = QueueN::<T, 32>::new();
        channel_with_queue(queue)
    }

    fn channel_with_queue<T>(queue: Queue<T>) -> (Sender<T>, Receiver<T>) {
        let sender = Arc::new(Inner {
            queue,
            wake_sender: AtomicWaker::new(),
            wake_receiver: AtomicWaker::new(),
            sender_count: AtomicUsize::new(1),
            is_sender_closed: AtomicBool::new(false),
        });
        let receiver = Arc::clone(&sender);
        (Sender(sender), Receiver(receiver))
    }

    struct Inner<T> {
        queue: Queue<T>,
        wake_sender: AtomicWaker,
        wake_receiver: AtomicWaker,
        sender_count: AtomicUsize,
        is_sender_closed: AtomicBool,
    }

    /// Sender
    pub struct Sender<T>(Arc<Inner<T>>);

    impl<T> Sender<T> {
        pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> {
            let mut value = Some(value);
            poll_fn(|context| {
                match self.0.queue.enqueue(value.take().unwrap()) {
                    Ok(()) => {
                        self.0.wake_receiver.wake();
                        Poll::Ready(Ok(()))
                    }
                    Err(_e) => {
                        // queue is full

                        // TODO how do tokio chan's behave in overflow?
                        {
                            self.0.is_sender_closed.swap(true, AcqRel);
                            self.0.wake_receiver.wake();
                        }

                        self.0.wake_sender.register(context.waker());
                        Poll::Pending
                    }
                }
            })
            .await
        }

        pub async fn closed(&self) {
            // nop
        }
    }

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            self.0.sender_count.fetch_add(1, Relaxed);
            Sender(self.0.clone())
        }
    }

    impl<T> Drop for Sender<T> {
        fn drop(&mut self) {
            let sender_count = self.0.sender_count.fetch_sub(1, AcqRel);
            if sender_count != 1 {
                return;
            }
            self.0.is_sender_closed.swap(true, AcqRel);
            self.0.wake_receiver.wake();
        }
    }

    impl<T> core::fmt::Debug for Sender<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "Sender")
        }
    }

    /// Receiver
    pub struct Receiver<T>(Arc<Inner<T>>);

    impl<T: core::fmt::Debug> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            poll_fn(|context| match self.0.queue.dequeue() {
                Some(value) => {
                    self.0.wake_sender.wake();
                    Poll::Ready(Some(value))
                }
                None => {
                    self.0.wake_receiver.register(context.waker());
                    if self.0.is_sender_closed.load(Relaxed) {
                        Poll::Ready(None)
                    } else {
                        Poll::Pending
                    }
                }
            })
            .await
        }
    }

    impl<T> core::fmt::Debug for Receiver<T> {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "[Receiver]")
        }
    }

    /// mpsc::error::SendError
    pub mod error {
        use core::fmt;
        use ockam_core::compat::error;

        #[derive(Debug)]
        pub struct SendError<T>(pub T);

        impl<T> fmt::Display for SendError<T> {
            fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "SendError -> channel closed")
            }
        }

        impl<T: fmt::Debug> error::Error for SendError<T> {}
    }

    #[cfg(all(feature = "std", test))]
    mod tests {
        use crate::tokio::sync::mpsc::{
            channel, channel_n, channel_with_queue, channel_with_queue_n, Queue, QueueN, Receiver,
            ReceiverN, Sender, SenderN,
        };
        use async_cortex_m::task::block_on;
        use async_cortex_m::task::spawn;
        use async_embedded as async_cortex_m;
        use ockam_core::compat::format;

        #[test]
        fn test_send_receive() {
            async {
                let queue = Queue::<u32>::new();
                let (tx, mut rx): (Sender<u32>, Receiver<u32>) = channel_with_queue(queue);

                let _result = tx.send(23).await;
                let x = rx.recv().await;
            };
        }

        #[test]
        fn test_spawn_one_sender() {
            async {
                let queue = heapless::mpmc::MpMcQueue::<u32, 32>::new();
                let (mut tx, mut rx) = channel_with_queue(queue);

                spawn(async move {
                    for i in 0..10 {
                        if let Err(_) = tx.send(i).await {
                            println!("receiver dropped");
                            return;
                        }
                    }
                });

                block_on(async move {
                    while let Some(i) = rx.recv().await {
                        println!("got = {}", i);
                    }
                });
            };
        }

        #[test]
        fn test_spawn_many_senders() {
            async {
                let (tx, mut rx) = channel(32);
                for n in 0..10 {
                    let tx2 = tx.clone();
                    spawn(async move {
                        tx2.send(format!("sent from: {}", n)).await.unwrap();
                    });
                }

                spawn(async move {
                    tx.send(format!("closing last tx")).await.unwrap();
                });

                block_on(async move {
                    while let Some(message) = rx.recv().await {
                        println!("GOT = {}", message);
                    }
                });
            };
        }

        #[test]
        fn test_queue_sizes() {
            let queue = QueueN::<u32, 32>::new();
            let (tx, mut rx) = channel_with_queue_n(queue);
            let (tx, mut rx): (SenderN<u32, 1>, ReceiverN<u32, 1>) = channel_n();
            let (tx, mut rx): (SenderN<u32, 32>, ReceiverN<u32, 32>) = channel_n();

            let queue = Queue::<u32>::new();
            let (tx, mut rx) = channel_with_queue(queue);
            let (tx, mut rx): (Sender<u32>, Receiver<u32>) = channel(1);
            let (tx, mut rx): (Sender<u32>, Receiver<u32>) = channel(32);
        }

        #[tokio::test]
        async fn test_tokio_spawn_one_sender() {
            let (tx, mut rx) = tokio::sync::mpsc::channel(32);
            tokio::spawn(async move {
                for n in 0..10 {
                    tx.send(format!("sent from: {}", n)).await.unwrap();
                }
            });

            while let Some(message) = rx.recv().await {
                println!("test_tokio_spawn_one_sender got: {}", message);
            }
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
        async fn test_tokio_spawn_many_senders() {
            let (tx, mut rx) = tokio::sync::mpsc::channel(32);
            for n in 0..10 {
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    tx2.send(format!("sent from: {}", n)).await.unwrap();
                });
            }

            tokio::spawn(async move {
                tx.send(format!("closing last tx")).await.unwrap();
            });

            while let Some(message) = rx.recv().await {
                println!("test_tokio_spawn_many_senders got: {}", message);
            }
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn test_tokio_overflow_queue() {
            let (tx, mut rx) = crate::tokio::sync::mpsc::channel(32);

            let tx2 = tx.clone();
            tokio::spawn(async move {
                for n in 0..100 {
                    let tx3 = tx2.clone();
                    tokio::spawn(async move {
                        tx3.send(format!("sent from: {}", n)).await.unwrap();
                    });
                }
            });

            tokio::spawn(async move {
                tx.send(format!("closing last tx")).await.unwrap();
            });

            while let Some(message) = rx.recv().await {
                println!("test_tokio_overflow_queue got: {}", message);
            }
        }
    }

    /// initial draft of statically sized channel implementation
    pub fn channel_n<T, const N: usize>() -> (SenderN<T, N>, ReceiverN<T, N>) {
        let queue = QueueN::<T, N>::new();
        channel_with_queue_n(queue)
    }

    fn channel_with_queue_n<T, const N: usize>(
        queue: QueueN<T, N>,
    ) -> (SenderN<T, N>, ReceiverN<T, N>) {
        let sender = Arc::new(queue);
        let receiver = Arc::clone(&sender);
        (SenderN(sender), ReceiverN(receiver))
    }

    /// SenderN
    pub struct SenderN<T, const N: usize>(Arc<QueueN<T, N>>);

    impl<T, const N: usize> SenderN<T, N> {
        pub async fn send(&self, value: T) -> Result<(), error::SendError<T>> {
            match self.0.enqueue(value) {
                Ok(()) => Ok(()),
                Err(e) => Err(error::SendError(e)),
            }
        }

        pub async fn closed(&self) {
            unimplemented!();
        }
    }

    impl<T, const N: usize> Clone for SenderN<T, N> {
        fn clone(&self) -> Self {
            unimplemented!();
        }
    }

    impl<T, const N: usize> core::fmt::Debug for SenderN<T, N> {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "SenderN")
        }
    }

    /// ReceiverN
    pub struct ReceiverN<T, const N: usize>(Arc<QueueN<T, N>>);

    impl<T, const N: usize> ReceiverN<T, N> {
        pub async fn recv(&mut self) -> Option<T> {
            poll_fn(|_context| match self.0.dequeue() {
                Some(item) => core::task::Poll::Ready(Some(item)),
                None => core::task::Poll::Pending,
            })
            .await
        }
    }

    impl<T, const N: usize> core::fmt::Debug for ReceiverN<T, N> {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "Receiver")
        }
    }
}
