use std::any::Any;
use std::sync::{Arc, Mutex};

use crate::{Address, Context, Node};

/// A message handling unit of work in the Ockam [`Node`].
pub trait Worker: Send {
    /// Lifecycle callback that runs when the [`Worker`] has been created.
    fn initializing(&mut self, _context: &Context) {}

    /// Lifecycle callback that runs when the [`Worker`] has been stopped.
    fn stopping(&mut self) {}

    /// Data handler. TODO, moved/changed
    fn handle(&mut self, _data: Box<dyn Any>, _context: &Context) {}
}

/// TODO moved/changed
pub trait Handler<T> {
    /// TODO moved/changed
    fn handle(&mut self, _data: T, _context: &Context) {}
}

/// A high level, thread safe wrapper type around a [`Worker`].
pub type WorkerHandle = Arc<Mutex<dyn Worker>>;

/// A fluent [`Worker`] builder.
pub struct WorkerBuilder {
    node: Option<Node>,
    worker: WorkerHandle,
    address: Option<Address>,
}

impl WorkerBuilder {
    /// Create a new [`WorkerBuilder`] from the [`Worker`] implementation.
    pub fn new(worker: impl Worker + 'static) -> Self {
        WorkerBuilder {
            worker: Arc::new(Mutex::new(worker)),
            node: None,
            address: None,
        }
    }

    /// The [`Node`] running this [`Worker`].
    pub fn on(&mut self, node: Node) -> &mut Self {
        self.node = Some(node);
        self
    }

    /// [`Address`] of the [`Worker`].
    pub fn at<S: ToString>(&mut self, address: S) -> &mut Self {
        self.address = Some(address.to_string());
        self
    }

    /// Create and start the [`Worker`]. Return the [`Address`] of the new [`Worker`].
    pub async fn start(&self) -> Option<Address> {
        if let Some(node) = &self.node {
            if let Some(address) = &self.address {
                node.create_worker(self.worker.clone(), address.to_string())
                    .await;
                return Some(address.clone());
            }
        }
        // TODO Error handling with nice user facing messaging
        None
    }
}
