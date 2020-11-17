use core::cell::RefCell;

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use alloc::collections::VecDeque;

use crate::Addressable;
use hashbrown::HashMap;

/// A wrapper around a byte vector, for storing payloads in [`Queue`]s. Needs to be enhanced with
/// meta-info like decryption state.
#[derive(Clone)]
pub struct QueueMessage {
    pub body: Vec<u8>,
}

pub trait ToMessage<T> {
    fn to_msg(&self) -> Option<T>;
}

impl QueueMessage {
    pub fn new(body: Vec<u8>) -> QueueMessage {
        QueueMessage { body }
    }
}

impl ToMessage<QueueMessage> for &[u8] {
    fn to_msg(&self) -> Option<QueueMessage> {
        Some(QueueMessage {
            body: self.to_vec(),
        })
    }
}

impl ToMessage<QueueMessage> for &str {
    fn to_msg(&self) -> Option<QueueMessage> {
        ToMessage::to_msg(&self.as_bytes())
    }
}

impl ToMessage<QueueMessage> for String {
    fn to_msg(&self) -> Option<QueueMessage> {
        ToMessage::to_msg(&self.as_str())
    }
}

pub trait Enqueue<T> {
    fn enqueue(&mut self, message: T);
}

pub trait Dequeue<T> {
    fn dequeue(&mut self) -> Option<T>;
}

pub trait QueueMeta {
    /// Returns true if the underlying queue has messages.
    fn has_messages(&self) -> bool;
}

pub trait Queue<T>: Enqueue<T> + Dequeue<T> + QueueMeta + Addressable {}

/// An in-memory [`Queue`] which stores [`QueueMessage`]s using a [`VecDeque`]. At most
/// `message_limit` messages will be stored. A `message_limit` of 0 disables the limit.
pub struct MemQueue {
    address: String,
    messages: VecDeque<QueueMessage>,
    message_limit: usize,
    dropped_messages: usize,
}

impl Enqueue<QueueMessage> for MemQueue {
    fn enqueue(&mut self, message: QueueMessage) {
        if self.message_limit == 0 || self.messages.len() < self.message_limit {
            self.messages.push_back(message)
        } else {
            self.dropped_messages += 1;
        }
    }
}

impl Dequeue<QueueMessage> for MemQueue {
    fn dequeue(&mut self) -> Option<QueueMessage> {
        match self.has_messages() {
            true => self.messages.pop_front(),
            false => None,
        }
    }
}

impl Addressable for MemQueue {
    fn address(&self) -> String {
        self.address.clone()
    }
}

impl Queue<QueueMessage> for MemQueue {}

impl QueueMeta for MemQueue {
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
pub trait QueueManagement {
    fn address(&self) -> String;

    /// Get the [`Queue`] associated with the `queue_address`, if it exists.
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle>;

    /// Remove the [`Queue`] at the given `queue_address`.
    fn remove_queue(&mut self, queue_address: &str);
}

/// Wrapper type for QueueWorker trait object.
pub type QueueWorkerHandle = Rc<RefCell<dyn QueueManagement>>;

/// An in-memory [`Queue`] tracking worker. Queues will be created on demand by `get_queue`.
pub struct MemQueueWorker {
    address: String,
    queue_map: HashMap<String, QueueHandle>,
    default_queue_limit: usize,
}

impl MemQueueWorker {
    pub fn new<T>(address: T, default_queue_limit: usize) -> MemQueueWorker
    where
        T: ToString,
    {
        MemQueueWorker {
            address: address.to_string(),
            queue_map: HashMap::new(),
            default_queue_limit,
        }
    }

    pub fn create<T>(address: T, default_queue_limit: usize) -> QueueWorkerHandle
    where
        T: ToString,
    {
        Rc::new(RefCell::new(MemQueueWorker::new(
            address,
            default_queue_limit,
        )))
    }

    pub fn create_unbound<T>(address: T) -> QueueWorkerHandle
    where
        T: ToString,
    {
        MemQueueWorker::create(address, 0)
    }
}

impl QueueManagement for MemQueueWorker {
    fn address(&self) -> String {
        self.address.clone()
    }

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
            queue.enqueue(i.to_string().to_msg().unwrap());
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

        queue.enqueue("a".to_msg().unwrap());
        assert_eq!(1, queue.len());
        queue.dequeue();
        assert_eq!(0, queue.len());

        for i in 0..limit {
            queue.enqueue((&i.to_string()).to_msg().unwrap());
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
        queue.enqueue("a".to_msg().unwrap());
        assert!(queue.has_messages());
        queue.dequeue();
        assert!(!queue.has_messages());
    }

    #[test]
    fn test_get_queue() {
        let queue_worker_handle = MemQueueWorker::create_unbound("qw1");
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
        let queue_worker_handle = MemQueueWorker::create_unbound("qw1");
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
        let queue_worker = MemQueueWorker::create_unbound("qw1");

        let queue_address = "test";
        {
            let mut qw = queue_worker.borrow_mut();
            let queue: QueueHandle = qw.get_queue(queue_address).unwrap();

            let mut rm = queue.borrow_mut();
            rm.enqueue("A".to_msg().unwrap());
            rm.enqueue("B".to_msg().unwrap());
            rm.enqueue("C".to_msg().unwrap());

            let _out_message = rm.dequeue().unwrap();
        }

        let mut qw = queue_worker.borrow_mut();
        qw.remove_queue(queue_address);
    }
}
