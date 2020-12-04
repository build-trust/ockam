use core::cell::RefCell;

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use alloc::collections::VecDeque;
use core::str::FromStr;
use hashbrown::HashMap;

/// A wrapper around a byte vector, for storing payloads in [`Queue`]s. Needs to be enhanced with
/// meta-info like decryption state.
#[derive(Clone)]
pub struct QueueMessage {
    pub body: Vec<u8>,
}

impl QueueMessage {
    pub fn new(body: Vec<u8>) -> QueueMessage {
        QueueMessage { body }
    }
}

impl FromStr for QueueMessage {
    type Err = u8;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(QueueMessage::new(s.as_bytes().to_vec()))
    }
}

/// An addressable FIFO queue which stores messages of type `T`.
pub trait Queue<T> {
    /// Returns the address of this queue.
    fn address(&self) -> &str;

    /// Enqueue a message of type `T` into the underlying queue.
    fn enqueue(&mut self, message: T);

    /// Attempt to dequeue a message of type `T` from the underlying queue.
    /// Returns `Some(T)` if there was a message in the queue.
    /// If the queue is empty, `None` is returned.
    fn dequeue(&mut self) -> Option<T>;

    /// Returns true if the underlying queue has messages.
    fn has_messages(&self) -> bool;
}

/// An in-memory [`Queue`] which stores [`QueueMessage`]s using a [`VecDeque`]. At most
/// `message_limit` messages will be stored. A `message_limit` of 0 disables the limit.
pub struct MemQueue {
    address: String,
    messages: VecDeque<QueueMessage>,
    message_limit: usize,
    dropped_messages: usize,
}

impl Queue<QueueMessage> for MemQueue {
    fn address(&self) -> &str {
        &self.address
    }

    fn enqueue(&mut self, message: QueueMessage) {
        if self.message_limit == 0 || self.messages.len() < self.message_limit {
            self.messages.push_back(message)
        } else {
            self.dropped_messages += 1;
        }
    }

    fn dequeue(&mut self) -> Option<QueueMessage> {
        match self.has_messages() {
            true => self.messages.pop_front(),
            false => None,
        }
    }

    fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }
}

/// Wrapper type for handling [`Queue`]<[`QueueMessage`]> trait objects.
pub type QueueHandle = Rc<RefCell<dyn Queue<QueueMessage>>>;

impl MemQueue {
    pub fn new<S>(address: S, message_limit: usize) -> MemQueue
    where
        S: ToString,
    {
        MemQueue {
            address: address.to_string(),
            messages: VecDeque::new(),
            dropped_messages: 0,
            message_limit,
        }
    }

    pub fn create<S>(address: S, message_limit: usize) -> QueueHandle
    where
        S: ToString,
    {
        Rc::new(RefCell::new(MemQueue::new(address, message_limit)))
    }

    pub fn create_unbound<S>(address: S) -> QueueHandle
    where
        S: ToString,
    {
        MemQueue::create(address, 0)
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

/// A trait representing a QueueWorker, which manages the addressing and storage of [`Queue`]s.
pub trait QueueWorker {
    /// Get the [`Queue`] associated with the `queue_address`, if it exists.
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle>;

    /// Remove the [`Queue`] at the given `queue_address`.
    fn remove_queue(&mut self, queue_address: &str);
}

/// Wrapper type for QueueWorker trait object.
pub type QueueWorkerHandle = Rc<RefCell<dyn QueueWorker>>;

/// An in-memory [`Queue`] tracking worker. Queues will be created on demand by `get_queue`.
pub struct MemQueueWorker {
    queue_map: HashMap<String, QueueHandle>,
    default_queue_limit: usize,
}

impl MemQueueWorker {
    pub fn new(default_queue_limit: usize) -> MemQueueWorker {
        MemQueueWorker {
            queue_map: HashMap::new(),
            default_queue_limit,
        }
    }

    pub fn create(default_queue_limit: usize) -> QueueWorkerHandle {
        Rc::new(RefCell::new(MemQueueWorker::new(default_queue_limit)))
    }

    pub fn create_unbound() -> QueueWorkerHandle {
        MemQueueWorker::create(0)
    }
}

impl QueueWorker for MemQueueWorker {
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle> {
        if queue_address.is_empty() {
            return None;
        }

        if self.queue_map.contains_key(queue_address) {
            self.queue_map.get(queue_address).cloned()
        } else {
            let name_string = queue_address.to_string();
            let new_queue = MemQueue::create(name_string.clone(), self.default_queue_limit);
            self.queue_map.insert(name_string.clone(), new_queue);
            self.queue_map.get(queue_address).cloned()
        }
    }

    fn remove_queue(&mut self, queue_name: &str) {
        self.queue_map.remove(queue_name);
    }
}

#[cfg(test)]
mod queue_tests {
    use crate::queue::*;

    const TEST_ADDRESS: &'static str = "worker_producer_ciphertext_0123";

    #[test]
    fn test_queue_address() {
        let queue = MemQueue::new(TEST_ADDRESS, 0);
        assert_eq!(TEST_ADDRESS, queue.address())
    }

    #[test]
    fn test_queue_enqueue() {
        let limit = 100;
        let mut queue = MemQueue::new(TEST_ADDRESS, limit);

        for i in 0..limit {
            queue.enqueue(QueueMessage::from_str(&i.to_string()).unwrap());
            assert_eq!(i + 1, queue.len())
        }
        assert_eq!(limit, queue.len())
    }

    #[test]
    fn test_dequeue() {
        let limit = 100;

        let mut queue = MemQueue::new(TEST_ADDRESS, limit);

        assert_eq!(0, queue.len());
        queue.dequeue();
        assert_eq!(0, queue.len());

        queue.enqueue(QueueMessage::from_str("a").unwrap());
        assert_eq!(1, queue.len());
        queue.dequeue();
        assert_eq!(0, queue.len());

        for i in 0..limit {
            queue.enqueue(QueueMessage::from_str(&i.to_string()).unwrap());
        }
        assert_eq!(limit, queue.len());

        let l = queue.len();

        // Ensure FIFO order is preserved
        for i in 0..l {
            let m = queue.dequeue().unwrap();
            assert_eq!(i.to_string().as_bytes().to_vec(), m.body)
        }
        assert_eq!(0, queue.len());
    }

    #[test]
    fn test_has_messages() {
        let mut queue = MemQueue::new(TEST_ADDRESS, 1);
        queue.enqueue(QueueMessage::from_str("a").unwrap());
        assert!(queue.has_messages());
        queue.dequeue();
        assert!(!queue.has_messages());
    }

    #[test]
    fn test_get_queue() {
        let mut queue_worker_handle = MemQueueWorker::create_unbound();
        let mut queue_worker = queue_worker_handle.borrow_mut();

        let blank_queue = queue_worker.get_queue("");
        assert!(blank_queue.is_none());

        let a = queue_worker.get_queue("a");
        assert!(a.is_some());

        let a2 = queue_worker.get_queue("a");
        assert!(a2.is_some());

        let queue_a = a.unwrap();
        let queue_a1 = a2.unwrap();
        assert_eq!(queue_a.borrow().address(), queue_a1.borrow().address());

        let b = queue_worker.get_queue("b");
        assert!(b.is_some());

        let queue_b = b.unwrap();

        assert_ne!(queue_a.borrow().address(), queue_b.borrow().address());
    }

    #[test]
    fn test_remove_queue() {
        let mut queue_worker_handle = MemQueueWorker::create_unbound();
        let mut queue_worker = queue_worker_handle.borrow_mut();

        queue_worker.remove_queue("");
        queue_worker.remove_queue("a");

        let a = queue_worker.get_queue("a");
        assert!(a.is_some());

        queue_worker.remove_queue("a");
        queue_worker.remove_queue("a");
    }

    #[test]
    fn queue_tdd() {
        let queue_worker = MemQueueWorker::create_unbound();

        let queue_address = "test";
        {
            let mut qw = queue_worker.borrow_mut();
            let queue: QueueHandle = qw.get_queue(queue_address).unwrap();

            let mut rm = queue.borrow_mut();
            rm.enqueue(QueueMessage::from_str("A").unwrap());
            rm.enqueue(QueueMessage::from_str("B").unwrap());
            rm.enqueue(QueueMessage::from_str("C").unwrap());

            let _out_message = rm.dequeue().unwrap();
        }

        let mut qw = queue_worker.borrow_mut();
        qw.remove_queue(queue_address);
    }
}
