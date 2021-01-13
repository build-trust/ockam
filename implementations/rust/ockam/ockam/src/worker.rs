use crate::address::Address;
use crate::message::{new_message_queue, Message};
use crate::node::{Node, WorkerContext};
use crate::queue::AddressableQueue;
use alloc::rc::Rc;
use core::cell::RefCell;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Started,
    Failed,
}

pub trait Worker<T> {
    fn handle(&self, _message: T, _context: &mut WorkerContext) -> crate::Result<bool> {
        unimplemented!()
    }

    fn starting(&mut self, _context: &mut WorkerContext) -> crate::Result<bool> {
        Ok(true)
    }

    fn stopping(&mut self, _context: &mut WorkerContext) -> crate::Result<bool> {
        Ok(true)
    }
}

struct ClosureWorker<T> {
    message_handler: Option<Rc<RefCell<dyn FnMut(&T, &mut WorkerContext)>>>,
}

impl<T> Worker<T> for ClosureWorker<T> {
    fn handle(&self, message: T, context: &mut WorkerContext) -> crate::Result<bool> {
        if let Some(handler) = self.message_handler.clone() {
            let mut h = handler.borrow_mut();
            h(&message, context);
            Ok(true)
        } else {
            Err(crate::Error::WorkerRuntime)
        }
    }
}

impl ClosureWorker<Message> {
    fn with_closure(
        f: impl FnMut(&Message, &mut WorkerContext) -> () + 'static,
    ) -> ClosureWorker<Message> {
        ClosureWorker {
            message_handler: Some(Rc::new(RefCell::new(f))),
        }
    }
}

pub struct WorkerBuilder {
    delegate: Option<Rc<RefCell<dyn Worker<Message>>>>,
    address: Option<Address>,
    mailbox: Option<Rc<RefCell<dyn AddressableQueue<Message>>>>,
    address_counter: usize,
    built: bool,
}

impl WorkerBuilder {
    pub fn address(&mut self, address_str: &str) -> &mut Self {
        self.address = Some(Address::from(address_str));
        self
    }

    pub fn mailbox(&mut self, mailbox: Rc<RefCell<dyn AddressableQueue<Message>>>) -> &mut Self {
        self.mailbox = Some(mailbox);
        self
    }

    pub fn build(&mut self) -> Option<Address> {
        if self.delegate.is_none() || self.address.is_none() {
            panic!("Tried to build Context with no address or worker")
        }

        if self.built {
            return self.address.clone();
        }

        let address = self.address.as_ref().cloned().unwrap();

        let delegate = self.delegate.as_ref().cloned().unwrap();

        let mailbox = if let Some(mbox) = &self.mailbox {
            (*mbox).clone()
        } else {
            new_message_queue(address.clone())
        };

        let mut maybe_node: Option<Rc<RefCell<Node>>> = None;

        crate::node::get(|n| maybe_node = Some(n.clone()));

        if let Some(node) = maybe_node {
            let worker_context = WorkerContext {
                delegate,
                inbox: mailbox.clone(),
                outbox: mailbox.clone(),
                node,
                address: address.clone(),
            };
            self.built = true;
            let addr = worker_context.inbox.borrow().address();

            crate::node::register(worker_context);

            Some(addr)
        } else {
            None
        }
    }

    pub fn start(&mut self) -> Option<Address> {
        match self.build() {
            Some(address) => {
                let state = crate::node::start(&address);
                if WorkerState::Started == state {
                    return Some(address);
                }
                None
            }
            _ => panic!(""),
        }
    }
}

pub fn with(worker: impl Worker<Message> + 'static) -> WorkerBuilder {
    let mut builder = WorkerBuilder {
        address: None,
        mailbox: None,
        address_counter: 1000,
        delegate: None,
        built: false,
    };

    builder.delegate = Some(Rc::new(RefCell::new(worker)));
    builder.address = Some(Address::new(builder.address_counter));
    builder
}

pub fn with_closure(
    handler: impl FnMut(&Message, &mut WorkerContext) -> () + 'static,
) -> WorkerBuilder {
    let closure = ClosureWorker::with_closure(handler);
    with(closure)
}

#[cfg(test)]
mod test {
    use crate::address::Address;
    use crate::message::Message;
    use crate::node::Node;
    use crate::queue::AddressedVec;
    use crate::worker::{ClosureWorker, WorkerContext};
    use alloc::collections::VecDeque;
    use alloc::rc::Rc;
    use core::cell::RefCell;

    #[test]
    fn worker_context() {
        let work = Rc::new(RefCell::new(
            |_message: &Message, _context: &mut WorkerContext| {},
        ));

        let mut context = WorkerContext {
            address: Address::from("test"),
            delegate: Rc::new(RefCell::new(ClosureWorker {
                message_handler: Some(work),
            })),
            inbox: Rc::new(RefCell::new(AddressedVec {
                address: Address::from("test_inbox"),
                vec: VecDeque::new(),
            })),
            outbox: Rc::new(RefCell::new(AddressedVec {
                address: Address::from("test_outbox"),
                vec: VecDeque::new(),
            })),
            node: Rc::new(RefCell::new(Node::new())),
        };

        let ctx = context.clone();

        let worker = ctx.delegate.borrow_mut();

        worker.handle(Message::empty(), &mut context).unwrap();
    }
}
