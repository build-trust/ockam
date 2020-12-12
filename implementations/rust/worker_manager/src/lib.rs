#![no_std]
extern crate alloc;
use ockam_no_std_traits::{ProcessMessage, Poll, ProcessMessageHandle, PollHandle, RouteMessageHandle};
use ockam_message::message::{Message};
use alloc::string::String;
use hashbrown::HashMap;
use alloc::collections::VecDeque;
use core::ops::Deref;
use libc_print::*;

pub struct WorkerManager {
    message_handlers: HashMap<String, ProcessMessageHandle>,
    poll_handlers: VecDeque<PollHandle>
}

impl WorkerManager {
    pub fn new() -> Self {
        WorkerManager { message_handlers: HashMap::new(), poll_handlers: VecDeque::new() }
    }

    pub fn register_worker(
        &mut self,
        address: String,
        message_handler: Option<ProcessMessageHandle>,
        poll_handler: Option<PollHandle>
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

impl ProcessMessage for WorkerManager {
    fn handle_message(&mut self, message: Message, q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
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
    fn poll(&mut self, q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
        libc_println!("Poll for WorkerManager");
        for p in self.poll_handlers.iter_mut() {
            let mut handler = p.deref().borrow_mut();
            handler.poll(q_ref.clone())?;
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
