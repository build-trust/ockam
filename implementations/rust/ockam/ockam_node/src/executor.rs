use std::future::Future;

use hashbrown::HashMap;
use ockam_core::Error;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub use command::*;

use crate::{Address, Worker, WorkerHandle};

use super::{Context, Node};
use std::sync::Arc;

mod command;

#[derive(Clone)]
pub struct NodeWorker<T> {
    context: Arc<Context<T>>,
    worker: WorkerHandle<T>,
}

impl<T> NodeWorker<T> {
    fn new(context: Context<T>, worker: WorkerHandle<T>) -> Self {
        NodeWorker {
            context: Arc::new(context),
            worker,
        }
    }
}

impl<T> Worker<T> for NodeWorker<T> {
    fn starting(&mut self, _context: &Context<T>) {
        if let Ok(mut worker) = self.worker.lock() {
            worker.starting(_context)
        }
    }

    fn stopping(&mut self) {
        if let Ok(mut worker) = self.worker.lock() {
            worker.stopping()
        }
    }

    fn handle(&mut self, _data: T, _context: &Context<T>) {
        println!("handle");
    }
}

pub struct NodeExecutor<T> {
    sender: Sender<Command<T>>,
    receiver: Receiver<Command<T>>,
    registry: HashMap<Address, NodeWorker<T>>,
}

impl<T> Default for NodeExecutor<T> {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let registry: HashMap<String, NodeWorker<T>> = HashMap::new();
        NodeExecutor {
            sender,
            receiver,
            registry,
        }
    }
}

impl<T> NodeExecutor<T> {
    pub fn new() -> Self {
        NodeExecutor::default()
    }

    pub fn new_worker_context<S: ToString>(&self, address: S) -> Context<T> {
        Context::new(Node::new(self.sender.clone()), address.to_string())
    }

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

    pub fn register_worker(&mut self, address: Address, mut worker: NodeWorker<T>) {
        let context = worker.context.clone();
        worker.starting(&context);

        self.registry.insert(address, worker);
    }

    pub fn has_registered_worker(&self, address: &str) -> bool {
        self.registry.contains_key(address)
    }

    pub fn unregister_worker(&mut self, address: &str) {
        self.registry.remove(address);
    }
}
