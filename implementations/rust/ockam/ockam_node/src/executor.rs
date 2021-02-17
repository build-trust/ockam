// use crate::message::BaseMessage;

use crate::{relay::RelayMessage, Context, Node, NodeMessage, NodeReply};
use ockam_core::{Address, Result};

use std::{collections::BTreeMap, future::Future, sync::Arc};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct Executor {
    /// Reference to the runtime needed to spawn tasks
    rt: Arc<Runtime>,
    /// Receiver for messages from node
    receiver: Receiver<NodeMessage>,
    /// Keeping a copy of node to clone and pass out
    node: Node,
    /// Worker handle map
    registry: BTreeMap<Address, Sender<RelayMessage>>,
}

impl Default for Executor {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let rt = Arc::new(Runtime::new().unwrap());
        let node = Node::new(sender, Arc::clone(&rt));
        let registry = BTreeMap::default();
        Self {
            rt,
            node,
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

    pub async fn receive(&mut self) -> Option<NodeMessage> {
        self.receiver.recv().await
    }

    /// Create a new [`Context`] at the given address.
    pub fn new_context<S: Into<Address>>(&self, address: S) -> Context {
        let node = self.node.clone();
        Context::new(node, address.into())
    }

    pub fn execute<F>(&mut self, future: F) -> Result<()>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let rt = Arc::clone(&self.rt);
        let _join = rt.spawn(future);
        rt.block_on(self.handle_incoming());

        Ok(())
    }

    async fn handle_incoming(&mut self) {
        while let Some(mut msg) = self.receive().await {
            match msg {
                NodeMessage::SenderReq(ref address, ref mut reply) => match self
                    .registry
                    .get(address)
                {
                    Some(sender) => reply.send(NodeReply::sender(address.clone(), sender.clone())),
                    None => reply.send(NodeReply::no_such_worker(address.clone())),
                }
                .await
                .unwrap(),
                NodeMessage::StopWorker(ref address, ref mut reply) => {
                    match self.registry.remove(address) {
                        Some(_) => reply.send(NodeReply::ok()),
                        None => reply.send(NodeReply::no_such_worker(address.clone())),
                    }
                    .await
                    .unwrap()
                }
                NodeMessage::StopNode => {
                    self.registry.clear(); // Dropping all senders stops all workers
                    break;
                }
                NodeMessage::StartWorker(address, sender) => {
                    // TODO: check that no worker with that address already exists?
                    self.registry.insert(address, sender);
                }
                NodeMessage::ListWorkers(sender) => {
                    let list = self.registry.keys().cloned().collect();
                    sender.send(NodeReply::Workers(list)).await.unwrap();
                }
            }
        }
    }
}
