use super::{Context, Node};

mod command;
pub use command::*;

use std::future::Future;

use ockam_core::Error;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub struct NodeExecutor {
    sender: Sender<Command>,
    receiver: Receiver<Command>,
    registry: HashMap<String, NodeWorker>,
}

impl NodeExecutor {
    pub fn new() -> Self {
        let (sender, receiver) = channel(32);
        NodeExecutor { sender, receiver }
    }

    pub fn new_worker_context(&self) -> Context {
        Context::new(Node::new(self.sender.clone()))
    }

    pub fn execute<T>(
        &mut self,
        application: impl Future<Output = T> + 'static + Send,
    ) -> Result<(), Error>
    where
        T: Send + 'static,
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

    pub fn register_worker(&mut self, address: String, worker: Context) {
        self.registry.insert(address, worker);
    }

    pub fn has_registered_worker(&self, address: &str) -> bool {
        self.registry.contains_key(address)
    }

    pub fn unregister_worker(&mut self, address: &str) {
        self.registry.remove(address);
    }
}
