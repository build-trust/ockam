use std::any::Any;
use std::future::Future;

use hashbrown::HashMap;
use ockam_core::Error;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::{Context, Node, Handler};
pub use command::*;

mod command;

/// Runtime environment for [`Node`] command execution.
pub struct NodeExecutor {
    sender: Sender<Command>,
    receiver: Receiver<Command>,
    registry: HashMap<String, (Context, Box<dyn Any>)>,
}

impl Default for NodeExecutor {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let registry = HashMap::new();
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

    pub fn register<T>(&mut self, address: String, handler: T)
    where
        T: Any,
    {
        let context = self.new_worker_context(address.clone());
        self.registry.insert(address, (context, Box::new(handler)));
    }


    pub fn send<M: 'static>(&mut self, address: String, message: M) {
        let (context, handler) = self.registry.get_mut(&address).unwrap();
        let h = handler.downcast_mut::<Box <dyn Handler<M>>>().unwrap();
        h.handle(context, message);


        // let (_c, _h) = self.get::<Box <dyn Handler<M>>>(address.to_string()).unwrap();
    }
}
