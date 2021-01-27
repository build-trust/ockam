use std::sync::{Arc, Mutex};

use crate::{Address, Context, Node};

pub trait Worker<T>: Send {
    fn starting(&mut self, _context: &Context<T>) {}
    fn stopping(&mut self) {}
    fn handle(&mut self, _data: T, _context: &Context<T>) {}
}

pub trait Handler<T> {
    fn handle(&mut self, _data: T, _context: &Context<T>) {}
}

pub type WorkerHandle<T> = Arc<Mutex<dyn Worker<T>>>;

pub struct WorkerBuilder<T> {
    node: Option<Node<T>>,
    worker: WorkerHandle<T>,
    address: Option<Address>,
}

impl<T> WorkerBuilder<T> {
    pub fn new(worker: impl Worker<T> + 'static) -> Self {
        WorkerBuilder {
            worker: Arc::new(Mutex::new(worker)),
            node: None,
            address: None,
        }
    }

    pub fn on(&mut self, node: Node<T>) -> &mut Self {
        self.node = Some(node);
        self
    }

    pub fn at<S: ToString>(&mut self, address: S) -> &mut Self {
        self.address = Some(address.to_string());
        self
    }

    pub async fn start(&self) -> Option<Address> {
        if let Some(node) = &self.node {
            if let Some(address) = &self.address {
                node.create_worker(self.worker.clone(), address.to_string())
                    .await;
                return Some(address.clone());
            }
        }
        None
    }
}
