#[cfg(feature = "ockam_node_no_std")]
pub use ockam_node_no_std::block_on;

#[cfg(feature = "ockam_node_std")]
pub use ockam_node_std::block_on;

use crate::address::{Address, Addressable};
use crate::worker::{Worker, WorkerContext, WorkerState};
use alloc::rc::Rc;
use core::cell::RefCell;

use hashbrown::HashMap;

#[derive(Clone)]
struct Message {}

#[derive(Clone)]
pub struct WorkerRegistry<T> {
    workers: HashMap<Address, WorkerContext<T>>,
}

pub type RegistryHandle<T> = Rc<RefCell<WorkerRegistry<T>>>;

pub trait Registry<T> {
    fn register(&mut self, element: T);

    fn get(&mut self, key: &Address) -> Option<&mut T>;
}

impl<T> WorkerRegistry<T> {
    fn new() -> RegistryHandle<T> {
        Rc::new(RefCell::new(WorkerRegistry {
            workers: HashMap::new(),
        }))
    }
}

impl<T> Registry<WorkerContext<T>> for WorkerRegistry<T> {
    fn register(&mut self, worker: WorkerContext<T>) {
        let address = worker.address();
        self.workers.insert(address, worker);
    }

    fn get(&mut self, key: &Address) -> Option<&mut WorkerContext<T>> {
        if let Some(worker) = self.workers.get_mut(key) {
            Some(worker)
        } else {
            None
        }
    }
}

pub struct Node<T> {
    pub workers: RegistryHandle<T>,
}

pub type NodeHandle<T> = Rc<RefCell<Node<T>>>;

impl<T> Node<T> {
    pub fn new() -> NodeHandle<T> {
        Rc::new(RefCell::new(Node {
            workers: WorkerRegistry::new(),
        }))
    }

    pub fn start(&self, address: &Address) -> WorkerState {
        {
            let mut reg = self.workers.borrow_mut();
            if let Some(worker) = reg.get(address) {
                match worker.starting(worker) {
                    Ok(x) => x,
                    Err(_) => panic!(),
                };
            }
        }

        WorkerState::Started
    }

    pub fn stop(&self, address: &Address) -> WorkerState {
        let mut reg = self.workers.borrow_mut();
        if let Some(worker) = reg.get(address) {
            match worker.stopping(worker) {
                Ok(x) => x,
                Err(_) => unimplemented!(),
            };
        }
        WorkerState::Started
    }

    pub fn send(&self, address: &Address, _data: T) {
        let mut reg = self.workers.borrow_mut();
        if let Some(worker) = reg.get(address) {
            match worker.handle(_data, worker) {
                Ok(x) => x,
                Err(_) => unimplemented!(),
            };
        }
    }
}

#[cfg(test)]
mod test {
    use crate::address::Addressable;
    use crate::node::{Node, Registry};
    use crate::worker::{Worker, WorkerContext};

    struct Thing {}

    struct ThingWorker {}

    impl Worker<Thing> for ThingWorker {
        fn starting(&self, _worker: &WorkerContext<Thing>) -> crate::Result<bool> {
            Ok(true)
        }

        fn stopping(&self, _worker: &WorkerContext<Thing>) -> crate::Result<bool> {
            Ok(true)
        }
    }

    #[test]
    fn test_node() {
        let node_handle = Node::<Thing>::new();

        let node_clone = node_handle.clone();
        let node = node_clone.borrow_mut();

        let worker = crate::worker::with(node_handle, ThingWorker {})
            .build()
            .unwrap();

        let address = worker.address().clone();
        node.workers.borrow_mut().register(worker);

        node.start(&address);
        node.stop(&address);
    }
}
