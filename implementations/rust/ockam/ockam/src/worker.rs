use crate::address::{Address, Addressable};
use crate::node::{NodeHandle, Registry};
use crate::queue::{new_queue, AddressableQueue, QueueHandle};
use alloc::rc::Rc;
use core::cell::RefCell;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Started,
    Failed,
}

/// Worker callbacks.
pub trait Worker<T> {
    fn handle(&self, _message: T, _worker: &WorkerContext<T>) -> crate::Result<bool> {
        unimplemented!()
    }

    fn starting(&self, _worker: &WorkerContext<T>) -> crate::Result<bool> {
        Ok(true)
    }

    fn stopping(&self, _worker: &WorkerContext<T>) -> crate::Result<bool> {
        Ok(true)
    }
}

pub type WorkerHandler<T> = Rc<RefCell<dyn Worker<T>>>;

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

impl<T> Worker<T> for WorkerContext<T> {
    fn handle(&self, message: T, context: &WorkerContext<T>) -> crate::Result<bool> {
        self.worker.borrow_mut().handle(message, context)
    }

    fn starting(&self, worker: &WorkerContext<T>) -> crate::Result<bool> {
        self.worker.borrow().starting(worker)
    }

    fn stopping(&self, worker: &WorkerContext<T>) -> crate::Result<bool> {
        self.worker.borrow().stopping(worker)
    }
}

/// Wrapper type for creating a Worker given only a closure.
type ClosureHandle<T> = Rc<RefCell<dyn FnMut(&T, &WorkerContext<T>)>>;
struct ClosureCallbacks<T> {
    message_handler: Option<ClosureHandle<T>>,
}

impl<T> Worker<T> for ClosureCallbacks<T> {
    fn handle(&self, message: T, context: &WorkerContext<T>) -> crate::Result<bool> {
        if let Some(handler) = self.message_handler.clone() {
            let mut h = handler.borrow_mut();
            h(&message, context);
            Ok(true)
        } else {
            Err(crate::Error::WorkerRuntime) // We should discuss public api error patterns.
        }
    }
}

impl<T> ClosureCallbacks<T> {
    fn with_closure(f: impl FnMut(&T, &WorkerContext<T>) + 'static) -> ClosureCallbacks<T> {
        ClosureCallbacks {
            message_handler: Some(Rc::new(RefCell::new(f))),
        }
    }
}

pub type Mailbox<T> = Rc<RefCell<dyn AddressableQueue<T>>>;

pub struct WorkerBuilder<T> {
    node: NodeHandle<T>,
    callbacks: Option<WorkerHandler<T>>,
    address: Option<Address>,
    inbox: Option<Mailbox<T>>,
    address_counter: usize,
}

impl<T: 'static> WorkerBuilder<T> {
    pub fn address(&mut self, address_str: &str) -> &mut Self {
        self.address = Some(Address::from(address_str));
        self
    }

    pub fn inbox(&mut self, mailbox: Mailbox<T>) -> &mut Self {
        self.inbox = Some(mailbox);
        self
    }

    pub fn build(&mut self) -> Option<WorkerContext<T>> {
        if self.callbacks.is_none() || self.address.is_none() {
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
            which_address = Some(external_inbox.borrow().address())
        } else {
            which_inbox = Some(default_queue);
        }

        if let Some(delegate) = self.callbacks.clone() {
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

    pub fn start(&mut self) -> Option<Address> {
        if let Some(worker) = self.build() {
            let address = worker.address.clone();
            let node = worker.node.clone();

            node.borrow().workers.borrow_mut().register(worker);

            node.borrow().start(&address);
            Some(address)
        } else {
            panic!("Unable to build and start Worker");
        }
    }
}

/// Build a new Worker from the given implementation of Message Callbacks.
pub fn with<T>(node: NodeHandle<T>, worker: impl Worker<T> + 'static) -> WorkerBuilder<T> {
    let mut builder = WorkerBuilder {
        address: None,
        inbox: None,
        address_counter: 1000,
        callbacks: None,
        node,
    };

    builder.callbacks = Some(Rc::new(RefCell::new(worker)));
    builder.address = Some(Address::new(builder.address_counter));
    builder
}

/// Build a Worker from a closure.
pub fn with_closure<T: 'static>(
    node: NodeHandle<T>,
    handler: impl FnMut(&T, &WorkerContext<T>) + 'static,
) -> WorkerBuilder<T> {
    let closure = ClosureCallbacks::with_closure(handler);
    with(node, closure)
}

#[cfg(test)]
mod test {
    use crate::address::Address;
    use crate::node::Node;
    use crate::queue::AddressedVec;
    use crate::worker::{ClosureCallbacks, WorkerContext};
    use alloc::collections::VecDeque;
    use alloc::rc::Rc;
    use core::cell::RefCell;

    #[derive(Clone)]
    struct Thing {}

    #[test]
    fn test_worker() {
        let node = Node::new();

        let work = Rc::new(RefCell::new(
            |_message: &Thing, _context: &WorkerContext<Thing>| {},
        ));

        let worker = WorkerContext {
            node,
            address: Address::from("test"),
            worker: Rc::new(RefCell::new(ClosureCallbacks {
                message_handler: Some(work),
            })),
            inbox: Rc::new(RefCell::new(AddressedVec {
                address: Address::from("test_inbox"),
                vec: VecDeque::new(),
            })),
        };

        let delegate = worker.worker.borrow_mut();

        match delegate.handle(Thing {}, &mut worker.clone()) {
            Ok(x) => x,
            Err(_) => panic!(),
        };
    }
}
