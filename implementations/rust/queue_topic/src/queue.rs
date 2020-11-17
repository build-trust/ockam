use core::cell::RefCell;

use alloc::rc::Rc;
use alloc::string::{ToString, String};
use alloc::vec::Vec;

use hashbrown::HashMap;
use alloc::collections::VecDeque;
use core::str::FromStr;

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

pub trait Queue<T> {
    fn address(&self) -> String;
    fn enqueue(&mut self, message: T);
    fn dequeue(&mut self) -> Option<T>;
    fn has_messages(&self) -> bool;
}

struct MemQueue {
    address: String,
    messages: VecDeque<QueueMessage>,
}

impl Queue<QueueMessage> for MemQueue {
    fn address(&self) -> String {
        self.address.clone()
    }

    fn enqueue(&mut self, message: QueueMessage) {
        self.messages.push_back(message);
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

pub type QueueHandle = Rc<RefCell<dyn Queue<QueueMessage>>>;

impl MemQueue {
    fn new<S>(address: S) -> MemQueue
    where
        S: ToString,
    {
        MemQueue {
            address: address.to_string(),
            messages: VecDeque::new(),
        }
    }

    fn create<S>(address: S) -> QueueHandle
    where
        S: ToString,
    {
        Rc::new(RefCell::new(MemQueue::new(address)))
    }
}

pub struct MemQueueWorker {
    queue_map: HashMap<String, QueueHandle>,
}

pub trait QueueWorker {
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle>;

    fn remove_queue(&mut self, queue_name: &str);
}

pub type QueueWorkerHandle = Rc<RefCell<dyn QueueWorker>>;

impl MemQueueWorker {
    pub fn new() -> MemQueueWorker {
        MemQueueWorker {
            queue_map: HashMap::new(),
        }
    }

    pub fn create() -> QueueWorkerHandle {
        Rc::new(RefCell::new(MemQueueWorker::new()))
    }
}

impl QueueWorker for MemQueueWorker {
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle> {
        if self.queue_map.contains_key(queue_address) {
            self.queue_map.get(queue_address).cloned()
        } else {
            let name_string = queue_address.to_string();
            let new_queue = MemQueue::create(name_string.clone());
            self.queue_map.insert(name_string.clone(), new_queue);
            self.queue_map.get(queue_address).cloned()
        }
    }

    fn remove_queue(&mut self, queue_name: &str) {
        self.queue_map.remove(queue_name);
    }
}

#[test]
fn queue_tdd() {
    let queue_worker = MemQueueWorker::create();

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
