use std::sync::{Arc, Mutex};

use crate::{Address, Context, Node};

pub trait Message {}

/// A message handling unit of work in the Ockam [`Node`].
pub trait Worker: Send {
    /// Lifecycle callback that runs when the [`Worker`] has been created.
    fn initialize(&mut self, _context: &mut Context) {}
}

pub trait Handler<M>: Worker {
    fn handle(&mut self, _context: &mut Context, _message: M) {}
}

pub trait WorkerHandler<M>: Worker + Handler<M> {}

/// A [`Worker`] builder.
pub struct WorkerBuilder<T> {
    pub handler: Arc<Mutex<dyn Handler<T>>>,
    pub node: Option<Node>,
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
