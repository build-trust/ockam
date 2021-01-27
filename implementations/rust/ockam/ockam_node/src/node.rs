use super::{Command, Context, NodeError, NodeExecutor};

use ockam_core::Error;
use std::future::Future;
use tokio::sync::mpsc::{channel, Sender};

#[derive(Clone, Debug)]
pub struct Node {
    sender: Sender<Command>,
}

impl Node {
    pub fn new(sender: Sender<Command>) -> Self {
        Self { sender }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        match self.sender.send(Command::stop()).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }

    pub async fn create_worker<T>(&self, w: impl Future<Output = T> + 'static + Send)
    where
        T: Send + 'static,
    {
        // TODO: move thsi into the node executor
        tokio::spawn(w);

        match self.sender.send(Command::create_worker()).await {
            _ => (),
            // Ok(()) => Ok(()),
            // Err(_e) => Err(NodeError::CouldNotStop.into()),
        }
    }
}

pub fn node() -> (Context, NodeExecutor) {
    let (node_sender, node_receiver) = channel(32);

    let node_executor = NodeExecutor::new(node_receiver);
    let context = Context::new(Node::new(node_sender));

    (context, node_executor)
}

// use std::collections::HashMap;
//
// #[derive(Debug)]
// pub struct Node {
//
// }
//
// impl Node {
//     pub fn new() -> Self {
//         Node {
//             registry: HashMap::new(),
//         }
//     }
//
//
//     pub fn has_registered_worker(&self, address: &str) -> bool {
//         self.registry.contains_key(address)
//     }
//
//     pub fn register_worker(&mut self, address: String, worker: WorkerContext) {
//         self.registry.insert(address, worker);
//     }
//
//     pub fn unregister_worker(&mut self, address: &str) {
//         self.registry.remove(address);
//     }
// }
//
// #[cfg(test)]
// mod test {
//     use super::*;
//
//     #[test]
//     fn can_be_created() {
//         let _node: Node = Node::new();
//     }
//
//     #[test]
//     fn can_register_a_worker() {
//         let mut node = Node::new();
//         node.register_worker(String::from("a"), WorkerContext {});
//     }
//
//     #[test]
//     fn can_unregister_a_worker() {
//         let mut node = Node::new();
//         node.register_worker(String::from("a"), WorkerContext {});
//         node.unregister_worker("a");
//     }
//
//     #[test]
//     fn can_unregister_a_worker_even_if_it_was_never_registered() {
//         let mut node = Node::new();
//         node.unregister_worker("a");
//     }
//
//     #[test]
//     fn does_not_have_worker_before_one_is_registered() {
//         let node = Node::new();
//         assert_eq!(node.has_registered_worker("a"), false);
//     }
//
//     #[test]
//     fn has_worker_after_it_is_registered() {
//         let mut node = Node::new();
//         assert_eq!(node.has_registered_worker("a"), false);
//         node.register_worker(String::from("a"), WorkerContext {});
//         assert_eq!(node.has_registered_worker("a"), true);
//     }
// }
