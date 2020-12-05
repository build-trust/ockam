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

pub struct  PrintWorker {
    address: String,
    text: String,
}

impl PrintWorker {
    pub fn new(address: String, text: String) -> Self {
        PrintWorker{ address, text }
    }
}

impl Poll for PrintWorker {
    fn poll(&mut self, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String> {
        libc_println!("{} is polling", self.text);
        let msg_text = "sent to you by PrintWorker".as_bytes();
        let mut onward_addresses = Vec::new();

        onward_addresses.push(RouterAddress::worker_router_address_from_str("aabbccdd".into()).unwrap());
        let mut return_addresses = Vec::new();
        return_addresses.push(RouterAddress::worker_router_address_from_str(&self.address).unwrap());
        let m = Message {
            onward_route: Route{ addresses: onward_addresses },
            return_route: Route{ addresses: return_addresses },
            message_type: MessageType::Payload,
            message_body: msg_text.to_vec(),
        };
        let mut q = q_ref.deref().borrow_mut();
        q.enqueue(m);
        Ok(true)
    }
}

impl MessageHandler for PrintWorker {
    fn handle_message(&mut self, message: Message, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String> {
        libc_println!("Printworker: {}", std::str::from_utf8(&message.message_body).unwrap());
        Ok(true)
    }
}