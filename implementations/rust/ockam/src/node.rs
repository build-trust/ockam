#[cfg(feature = "ockam_node_no_std")]
pub use ockam_node_no_std::block_on;

#[cfg(feature = "ockam_node_std")]
pub use ockam_node_std::block_on;

use crate::address::Address;
use crate::message::Message;
use crate::queue::AddressableQueue;
use crate::worker::{Worker, WorkerState};
use alloc::rc::Rc;
use core::cell::RefCell;
use hashbrown::HashMap;

impl Worker<Message> for WorkerContext {
    fn handle(&self, message: Message, context: &mut WorkerContext) -> crate::Result<bool> {
        self.delegate.borrow_mut().handle(message, context)
    }
}

pub struct Sender {}

impl Sender {
    pub fn send(context: &mut WorkerContext, message: Message) {
        let mut mailbox = context.inbox.borrow_mut();
        if let Err(e) = mailbox.enqueue(message) {
            panic!("Couldn't enqueue message: {:?}", e)
        }
    }
}

#[derive(Clone)]
pub struct WorkerContext {
    pub delegate: Rc<RefCell<dyn Worker<Message>>>,
    pub inbox: Rc<RefCell<dyn AddressableQueue<Message>>>,
    pub outbox: Rc<RefCell<dyn AddressableQueue<Message>>>,
    pub node: Rc<RefCell<Node>>,
    pub address: Address,
}

impl WorkerContext {
    pub fn address(&self) -> Address {
        self.address.clone()
    }

    pub fn route(&self, message: Message) {
        self.node.borrow_mut().route(message);
    }
}

trait MessageDelivery {
    fn deliver(&mut self);
}

impl MessageDelivery for WorkerContext {
    fn deliver(&mut self) {
        if self.inbox.borrow().is_empty() {
            return;
        }

        let mut mbox = self.inbox.borrow_mut();
        while !mbox.is_empty() {
            if let Some(message) = mbox.dequeue() {
                let delegate = self.delegate.borrow_mut();
                let mut context = self.clone();
                delegate.handle(message, &mut context).unwrap();
            }
        }
    }
}

pub struct WorkerRegistry {
    workers: HashMap<Address, WorkerContext>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        WorkerRegistry {
            workers: HashMap::new(),
        }
    }

    pub fn insert(&mut self, context: WorkerContext) {
        let mailbox = context.inbox.borrow();
        let address = mailbox.address();
        self.workers.insert(address.clone(), context.clone());
    }

    pub fn get(&mut self, address: &Address) -> Option<&WorkerContext> {
        self.workers.get(address)
    }

    pub fn get_mut(&mut self, address: &Address) -> Option<&mut WorkerContext> {
        self.workers.get_mut(address)
    }

    pub fn with_worker(
        &mut self,
        address: &Address,
        mut handler: impl FnMut(Option<&mut WorkerContext>) -> (),
    ) {
        handler(self.workers.get_mut(address));
    }
}

impl MessageDelivery for WorkerRegistry {
    fn deliver(&mut self) {
        let workers = self.workers.values_mut();
        for worker in workers {
            worker.deliver();
        }
    }
}

fn start_on_node(node: &RefCell<Node>, address: &Address) -> WorkerState {
    let n = node.borrow();
    let mut registry = n.worker_registry.borrow_mut();
    if let Some(context) = registry.get_mut(address) {
        if let Ok(started) = context.delegate.borrow_mut().starting(&mut context.clone()) {
            if started {
                return WorkerState::Started;
            }
        }
    };
    WorkerState::Failed
}

fn send_on_node(node: &RefCell<Node>, address: &Address, message: Message) {
    let n = node.borrow();
    let mut registry = n.worker_registry.borrow_mut();
    if let Some(context) = registry.get_mut(address) {
        Sender::send(context, message);
    } else {
        panic!("No context at address {}", address)
    }
}

fn register_on_node(node: &RefCell<Node>, context: WorkerContext) {
    let n = node.borrow();
    let mut registry = n.worker_registry.borrow_mut();
    registry.insert(context);
}

pub struct Node {
    worker_registry: RefCell<WorkerRegistry>,
}

pub enum NodeErr {
    Internal,
}

pub type NodeResult<T> = core::result::Result<T, NodeErr>;

impl Node {
    pub(crate) fn new() -> Self {
        Node {
            worker_registry: RefCell::new(WorkerRegistry::new()),
        }
    }

    pub fn route(&mut self, _message: Message) {}

    pub fn register(&mut self, worker: WorkerContext) {
        self.worker_registry.borrow_mut().insert(worker);
    }
}

impl MessageDelivery for Node {
    fn deliver(&mut self) {
        self.worker_registry.borrow_mut().deliver();
    }
}

// std dependencies, highest level api
thread_local! {
    pub static NODE : Rc<RefCell<Node>> = Rc::new(RefCell::new(Node::new()))
}

pub fn get(f: impl FnOnce(&Rc<RefCell<Node>>) -> ()) {
    NODE.with(|node| f(&node))
}

pub fn start(address: &Address) -> WorkerState {
    NODE.with(|node| start_on_node(node, address))
}

pub fn send(address: &Address, message: Message) {
    NODE.with(|node| send_on_node(node, address, message));
    deliver();
}

pub fn route(message: Message) {
    NODE.with(|node| node.borrow_mut().route(message));
    deliver();
}

pub fn register(context: WorkerContext) {
    NODE.with(|node| {
        register_on_node(node, context);
    });
}

pub fn worker_at(address: &Address, handler: impl FnMut(Option<&mut WorkerContext>) -> ()) {
    NODE.with(|node| {
        node.borrow_mut()
            .worker_registry
            .borrow_mut()
            .with_worker(address, handler);
    })
}

pub fn deliver() {
    NODE.with(|node| node.borrow_mut().deliver())
}
