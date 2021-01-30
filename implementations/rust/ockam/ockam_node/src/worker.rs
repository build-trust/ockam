use std::sync::{Arc, Mutex};

use crate::{Address, Context, Node};

/// A message within the Ockam network.
pub trait Message {}

/// A message handling unit of work in the Ockam [`Node`].
pub trait Worker: Send {
    /// Lifecycle callback that runs when the [`Worker`] has been created.
    fn initialize(&mut self, _context: &mut Context) {}
}

/// Worker message handler interface.
pub trait Handler<M>: Worker {
    /// Handle a message of type `M`
    fn handle(&mut self, _context: &mut Context, _message: M) {}
}

/// High level trait for a Worker that can handle data.
pub trait WorkerHandler<M>: Worker + Handler<M> {}

/// A [`Worker`] builder.
pub struct WorkerBuilder<T> {
    /// Data handler.
    pub handler: Arc<Mutex<dyn Handler<T>>>,
    /// [`Node`] to run on.
    pub node: Option<Node>,
    /// [`Address`] of the Worker.
    pub address: Option<Address>,
}

impl<T: 'static> WorkerBuilder<T> {
    /// Create a new [`WorkerBuilder`] from the [`Worker`] implementation.
    pub fn new(handler: impl Handler<T> + 'static) -> Self {
        WorkerBuilder {
            handler: Arc::new(Mutex::new(handler)),
            node: None,
            address: None,
        }
    }

    /// The [`Node`] running this [`Worker`].
    pub fn on(&mut self, node: &Node) -> &mut Self {
        self.node = Some(node.clone());
        self
    }

    /// [`Address`] of the [`Worker`].
    pub fn at<S: ToString>(&mut self, address: S) -> &mut Self {
        self.address = Some(address.to_string());
        self
    }

    /// Create and start the [`Worker`]. Return the [`Address`] of the new [`Worker`].
    pub async fn start(&mut self) -> Option<Address> {
        if let Some(node) = &self.node {
            if let Some(address) = &self.address {
                node.start_worker(self.handler.clone(), address.to_string())
                    .await;
                return Some(address.clone());
            }
        }
        // TODO Error handling with nice user facing messaging
        None
    }
}
