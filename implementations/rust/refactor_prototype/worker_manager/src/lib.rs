#![no_std]
extern crate alloc;
use alloc::vec::*;
use alloc::rc::Rc;
use core::cell::RefCell;
use ockam_no_std_traits::{MessageHandler, Enqueue, Poll};
use ockam_message::message::{Message, Route, RouterAddress, MessageType};
use alloc::string::String;
use hashbrown::HashMap;
use alloc::collections::VecDeque;
use core::ops::Deref;
use libc_print::*;

pub struct WorkerManager {
    message_handlers: HashMap<String, Rc<RefCell<dyn MessageHandler>>>,
    poll_handlers: VecDeque<Rc<RefCell<dyn Poll>>>
}

impl WorkerManager {
    pub fn new() -> Self {
        WorkerManager { message_handlers: HashMap::new(), poll_handlers: VecDeque::new() }
    }

    pub fn register_worker(
        &mut self,
        address: String,
        message_handler: Option<Rc<RefCell<dyn MessageHandler>>>,
        poll_handler: Option<Rc<RefCell<dyn Poll>>>
    ) -> Result<bool, String> {
        libc_println!("registered {:?}", address);
        if let Some(mh) = message_handler {
            self.message_handlers.insert(address, mh);
        }
        if let Some(ph) = poll_handler {
            self.poll_handlers.push_back(ph);
        }
        Ok(true)
    }

}

impl MessageHandler for WorkerManager {
    fn handle_message(&mut self, message: Message, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String> {
        let address = message.onward_route.addresses[0].address.as_string();
        if let Some(h) = self.message_handlers.get_mut(&address) {
            let mut handler = h.deref().borrow_mut();
            handler.handle_message(message, q_ref)
        } else {
            Err("message handler not found".into())
        }
    }
}

impl Poll for WorkerManager {
    fn poll(&mut self, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String> {
        libc_println!("Poll for WorkerManager");
        for p in self.poll_handlers.iter_mut() {
            let mut handler = p.deref().borrow_mut();
            handler.poll(q_ref.clone());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
