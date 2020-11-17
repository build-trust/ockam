use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub struct QueueMessage {
    pub body: Vec<u8>,
}

impl QueueMessage {
    pub fn new(body: Vec<u8>) -> QueueMessage {
        QueueMessage { body }
    }

    pub fn from_str(s: &str) -> QueueMessage {
        QueueMessage::new(s.as_bytes().to_vec())
    }
}

pub trait Queue {
    fn address(&self) -> String;
    fn enqueue(&mut self, message: QueueMessage);
    fn dequeue(&mut self) -> Option<QueueMessage>;
}

struct MemQueue {
    address: String,
    messages: Vec<QueueMessage>,
}

impl Queue for MemQueue {
    fn address(&self) -> String {
        self.address.clone()
    }

    fn enqueue(&mut self, message: QueueMessage) {
        self.messages.push(message);
    }

    fn dequeue(&mut self) -> Option<QueueMessage> {
        match self.messages.len() != 0 {
            true => Some(self.messages.remove(0)),
            false => None,
        }
    }
}

pub type QueueHandle = Rc<RefCell<Box<dyn Queue>>>;

impl MemQueue {
    fn new<S>(address: S) -> MemQueue
    where
        S: ToString,
    {
        MemQueue {
            address: address.to_string(),
            messages: Vec::new(),
        }
    }

    fn create<S>(address: S) -> QueueHandle
    where
        S: ToString,
    {
        Rc::new(RefCell::new(Box::new(MemQueue::new(address))))
    }
}

pub struct MemQueueWorker {
    queue_map: HashMap<String, QueueHandle>,
}

pub trait QueueWorker {
    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle>;

    fn remove_queue(&mut self, queue_name: &str);
}

pub type QueueWorkerHandle = RefCell<Box<dyn QueueWorker>>;

impl MemQueueWorker {
    pub fn new() -> MemQueueWorker {
        MemQueueWorker {
            queue_map: HashMap::new(),
        }
    }

    pub fn create() -> QueueWorkerHandle {
        RefCell::new(Box::new(MemQueueWorker::new()))
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
        rm.enqueue(QueueMessage::from_str("A"));
        rm.enqueue(QueueMessage::from_str("B"));
        rm.enqueue(QueueMessage::from_str("C"));

        let out_message = rm.dequeue().unwrap();
        println!("{:?}", out_message.body);
    }

    let mut qw = queue_worker.borrow_mut();
    qw.remove_queue(queue_address);
}
