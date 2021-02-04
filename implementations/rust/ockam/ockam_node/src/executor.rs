use crate::message::Message;
use crate::node::Node;
use crate::Context;

use ockam_core::{Address, Result, Worker};

use std::collections::HashMap;
use std::future::Future;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct Executor {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    registry: HashMap<Address, (Context, Box<dyn Worker<Context = Context>>)>,
}

impl Default for Executor {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let registry = HashMap::new();
        Self {
            sender,
            receiver,
            registry,
        }
    }
}

impl Executor {
    /// Create a new [`Executor`].
    pub fn new() -> Self {
        Executor::default()
    }

    pub async fn receive(&mut self) -> Option<Message> {
        self.receiver.recv().await
    }

    /// Create a new [`Context`] at the given address.
    pub fn new_context<S: ToString>(&self, address: S) -> Context {
        let node = Node::new(self.sender.clone());
        Context::new(node, address.to_string())
    }

    pub fn execute<F>(&mut self, future: F) -> Result<()>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let runtime = Runtime::new().unwrap();
        let _join = runtime.spawn(future);
        runtime.block_on(Message::handle(self));

        Ok(())
    }

    /// Register a Handler at an address.
    pub fn register<S: ToString>(
        &mut self,
        address: S,
        worker: Box<dyn Worker<Context = Context>>,
    ) -> Result<()> {
        let address = address.to_string();
        let context = self.new_context(address.clone());
        self.registry.insert(address.clone(), (context, worker));

        let (context, w) = self.registry.get_mut(&address).unwrap();
        w.initialize(&mut context.clone()).unwrap();

        Ok(())
    }

    // pub fn send<M: 'static, S: ToString>(&mut self, s: S, message: M) -> Result<()> {
    //     let address = s.to_string();
    //     let (context, handler) = self.registry.get_mut(&address).unwrap();
    //     let h = handler.downcast_mut::<Box<dyn Handler<M, Context = Context>>>().unwrap();
    //     h.handle(message, &context);
    //
    //     Ok(())
    // }
}
