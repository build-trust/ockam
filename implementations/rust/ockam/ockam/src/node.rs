#[cfg(feature = "ockam_node_no_std")]
pub use ockam_node_no_std::block_on;

#[cfg(feature = "ockam_node_std")]
pub use ockam_node_std::block_on;

use crate::address::Address;
use crate::worker::{Registry, RegistryHandle, Worker, WorkerRegistry};

use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Message {}

#[derive(Clone)]
pub struct Node<T> {
    pub workers: RegistryHandle<T>,
}

pub type NodeHandle<T> = Arc<Mutex<Node<T>>>;

impl<T> Node<T> {
    pub fn new() -> NodeHandle<T> {
        Arc::new(Mutex::new(Node {
            workers: WorkerRegistry::new(),
        }))
    }

    pub fn send(&self, address: &Address, _data: T) {
        if let Ok(mut workers) = self.workers.lock() {
            if let Some(worker) = workers.get(address) {
                match worker.handle(_data, worker) {
                    Ok(x) => x,
                    Err(_) => unimplemented!(),
                };
            }
        }
    }
}
