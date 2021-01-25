use crate::address::{Address, Addressable};
use crate::node::NodeHandle;
use crate::queue::{new_queue, QueueHandle};
use crate::Error::WorkerRuntime;
use hashbrown::HashMap;
use ockam_error::OckamResult;
use std::sync::{Arc, Mutex};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Started,
    Failed,
}

// Starting and stopping callbacks.
pub trait Starting<T> {
    fn starting(&self, _worker: &WorkerContext<T>) -> OckamResult<bool> {
        Ok(true)
    }
}

pub trait Stopping<T> {
    fn stopping(&self, _worker: &WorkerContext<T>) -> OckamResult<bool> {
        Ok(true)
    }
}

/// Data handler callback.
pub trait Handler<T> {
    fn handle(&self, _message: T, _worker: &WorkerContext<T>) -> OckamResult<bool> {
        Ok(true)
    }
}

pub trait Worker<T>: Starting<T> + Stopping<T> + Handler<T> {}

pub type WorkerHandler<T> = Arc<Mutex<dyn Worker<T> + Send>>;

/// High level Worker.
#[derive(Clone)]
pub struct WorkerContext<T> {
    address: Address,
    pub worker: WorkerHandler<T>,
    pub inbox: QueueHandle<T>,
    pub node: NodeHandle<T>,
}

impl<T> Addressable for WorkerContext<T> {
    fn address(&self) -> Address {
        self.address.clone()
    }
}

impl<T> Starting<T> for WorkerContext<T> {}

impl<T> Stopping<T> for WorkerContext<T> {}

impl<T> Handler<T> for WorkerContext<T> {
    fn handle(&self, message: T, context: &WorkerContext<T>) -> OckamResult<bool> {
        if let Ok(worker) = self.worker.lock() {
            worker.handle(message, context)
        } else {
            Err(WorkerRuntime.into())
        }
    }
}

pub type Mailbox<T> = QueueHandle<T>;

pub struct WorkerBuilder<T> {
    node: NodeHandle<T>,
    delegate: Option<WorkerHandler<T>>,
    address: Option<Address>,
    inbox: Option<Mailbox<T>>,
    address_counter: usize,
}

impl<T: 'static + Send> WorkerBuilder<T> {
    pub fn address(&mut self, address_str: &str) -> &mut Self {
        self.address = Some(Address::from(address_str));
        self
    }

    pub fn inbox(&mut self, mailbox: Mailbox<T>) -> &mut Self {
        self.inbox = Some(mailbox);
        self
    }

    pub fn build(&mut self) -> Option<WorkerContext<T>> {
        if self.delegate.is_none() || self.address.is_none() {
            panic!("Tried to build Context with no Worker or Address")
        }

        let mut which_address = self.address.clone();
        let mut which_inbox = self.inbox.clone();

        let default_queue = new_queue(format!(
            "{}_in",
            match self.address.clone() {
                Some(x) => x,
                None => panic!(),
            }
        ));

        if let Some(external_inbox) = which_inbox.clone() {
            if let Ok(ext) = external_inbox.lock() {
                which_address = Some(ext.address());
            }
        } else {
            which_inbox = Some(default_queue);
        }

        if let Some(delegate) = self.delegate.clone() {
            if let Some(address) = which_address {
                if let Some(inbox) = which_inbox {
                    return Some(WorkerContext {
                        node: self.node.clone(),
                        address,
                        worker: delegate,
                        inbox,
                    });
                }
            }
        }
        None
    }

    pub async fn start(&mut self) -> Option<Address> {
        if let Some(worker) = self.build() {
            let address = worker.address.clone();
            let node = worker.node.clone();

            if let Ok(n) = node.lock() {
                if let Ok(mut workers) = n.workers.lock() {
                    workers.register(worker);
                }
            }

            Some(address)
        } else {
            panic!("Unable to build and start Worker");
        }
    }
}

#[derive(Clone)]
pub struct WorkerRegistry<T> {
    workers: HashMap<Address, WorkerContext<T>>,
}

pub type RegistryHandle<T> = Arc<Mutex<WorkerRegistry<T>>>;

pub trait Registry<T> {
    fn register(&mut self, element: T);

    fn get(&mut self, key: &Address) -> Option<&mut T>;
}

impl<T> WorkerRegistry<T> {
    pub(crate) fn new() -> RegistryHandle<T> {
        Arc::new(Mutex::new(WorkerRegistry {
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

/// Build a new Worker from the given implementation of Message Callbacks.
pub fn with<T>(node: NodeHandle<T>, worker: impl Worker<T> + 'static + Send) -> WorkerBuilder<T> {
    let mut builder = WorkerBuilder {
        address: None,
        inbox: None,
        address_counter: 1000,
        delegate: None,
        node,
    };

    builder.delegate = Some(Arc::new(Mutex::new(worker)));
    builder.address = Some(Address::new(builder.address_counter));
    builder
}
