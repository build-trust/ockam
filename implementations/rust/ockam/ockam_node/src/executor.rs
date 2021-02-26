// use crate::message::BaseMessage;

use crate::{relay::RelayMessage, NodeMessage, NodeReply};
use ockam_core::{Address, AddressSet, Result};

use std::{collections::BTreeMap, future::Future, sync::Arc};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct Executor {
    /// Reference to the runtime needed to spawn tasks
    rt: Arc<Runtime>,
    /// Receiver for messages from node
    receiver: Receiver<NodeMessage>,
    /// Keeping a copy of the channel sender to pass out
    sender: Sender<NodeMessage>,
    /// Worker handle map
    registry: BTreeMap<Address, Sender<RelayMessage>>,
    /// Additional address map
    ///
    /// Each worker has a primary address, with secondary addresses
    /// that are stored in this map.  When shutting down a worker,
    /// secondary address senders also need to be cleared for the
    /// worker to shut down
    addr_map: BTreeMap<Address, AddressSet>,
}

impl Default for Executor {
    fn default() -> Self {
        let (sender, receiver) = channel(32);
        let rt = Arc::new(Runtime::new().unwrap());
        let registry = BTreeMap::default();
        let addr_map = BTreeMap::default();
        Self {
            rt,
            receiver,
            sender,
            registry,
            addr_map,
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

    pub(crate) fn sender(&self) -> Sender<NodeMessage> {
        self.sender.clone()
    }

    pub(crate) fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }

    /// Initialize the root application worker
    pub fn initialize_system<S: Into<Address>>(
        &mut self,
        address: S,
        mailbox: Sender<RelayMessage>,
    ) {
        self.registry.insert(address.into(), mailbox);
    }

    pub fn execute<F>(&mut self, future: F) -> Result<()>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let rt = Arc::clone(&self.rt);
        let _join = rt.spawn(future);

        // We may want to let handle_incoming return results to the
        // user about critical failures that occured running the node
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
                    let addresses = self.addr_map.remove(address).unwrap();

                    match addresses.iter().fold(Some(()), |opt, addr| {
                        match (opt, self.registry.remove(addr)) {
                            (Some(_), Some(_)) => Some(()),
                            (Some(_), None) => None,
                            (None, _) => None,
                        }
                    }) {
                        Some(_) => reply.send(NodeReply::ok()),
                        None => reply.send(NodeReply::no_such_worker(address.clone())),
                    }
                    .await
                    .unwrap();
                }
                NodeMessage::StopNode => {
                    self.registry.clear(); // Dropping all senders stops all workers
                    break;
                }
                NodeMessage::StartWorker(address, sender) => {
                    // TODO: check that no worker with that address already exists?
                    address.iter().for_each(|address| {
                        self.registry.insert(address.clone(), sender.clone());
                    });
                    self.addr_map.insert(address.first(), address);
                }
                NodeMessage::ListWorkers(sender) => {
                    let list = self.registry.keys().cloned().collect();
                    sender.send(NodeReply::Workers(list)).await.unwrap();
                }
            }
        }
    }
}
