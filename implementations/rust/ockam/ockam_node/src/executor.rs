use std::any::Any;
use std::future::Future;
use std::sync::Arc;

use hashbrown::HashMap;
use ockam_core::Error;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub use command::*;

use crate::{Address, Worker, WorkerHandle};

use super::{Context, Node};

mod command;

/// Binds a [`Context`] and [`Worker`] implementation together.
#[derive(Clone)]
pub struct NodeWorker {
    context: Arc<Context>,
    worker: WorkerHandle,
}

impl NodeWorker {
    fn new(context: Context, worker: WorkerHandle) -> Self {
        NodeWorker {
            context: Arc::new(context),
            worker,
        }
    }
}

impl Worker for NodeWorker {
    fn initializing(&mut self, _context: &Context) {
        if let Ok(mut worker) = self.worker.lock() {
            worker.initializing(_context)
        }
    }

    fn stopping(&mut self) {
        if let Ok(mut worker) = self.worker.lock() {
            worker.stopping()
        }
    }

    fn handle(&mut self, _data: Box<dyn Any>, _context: &Context) {
        println!("handle");
    }
}

/// Runtime environment for [`Node`] command execution.
pub struct NodeExecutor {
    sender: Sender<Command>,
    receiver: Receiver<Command>,
    registry: HashMap<Address, NodeWorker>,
}

impl Default for NodeExecutor {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let registry: HashMap<String, NodeWorker> = HashMap::new();
        NodeExecutor {
            sender,
            receiver,
            registry,
        }
    }
}

impl NodeExecutor {
    /// Create a new [`NodeExecutor`].
    pub fn new() -> Self {
        NodeExecutor::default()
    }

    /// Create a new [`Context`] for a [`Worker`] at the given [`Address`].
    pub fn new_worker_context<S: ToString>(&self, address: S) -> Context {
        Context::new(Node::new(self.sender.clone()), address.to_string())
    }

    /// Execute a stream of [`Command`]s. This function blocks until a [`Command`] signals a request
    /// to break, by returning `true`.
    pub fn execute<S>(
        &mut self,
        application: impl Future<Output = S> + 'static + Send,
    ) -> Result<(), Error>
    where
        S: Send + 'static,
    {
        let runtime = Runtime::new().unwrap();

        // TODO: turn app into a worker with an address
        runtime.spawn(application);

        runtime.block_on(async move {
            loop {
                if let Some(command) = self.receiver.recv().await {
                    let should_break = command.run(self);
                    if should_break {
                        break;
                    };
                }
            }
        });

        Ok(())
    }

    /// Register a [`Worker`] at the given [`Address`].
    pub fn register_worker(&mut self, address: Address, mut worker: NodeWorker) {
        let context = worker.context.clone();
        worker.initializing(&context);

        self.registry.insert(address, worker);
    }

    /// Returns true if there is a [`Worker`] associated with the given [`Address`].
    pub fn has_registered_worker(&self, address: &str) -> bool {
        self.registry.contains_key(address)
    }

    /// Remove a [`Worker`] from the registry.
    pub fn unregister_worker(&mut self, address: &str) {
        self.registry.remove(address);
    }
}
