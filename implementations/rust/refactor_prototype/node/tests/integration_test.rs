use ockam_node::{Node};
extern crate alloc;
use alloc::vec::*;
use alloc::rc::Rc;
use core::cell::RefCell;
use ockam_no_std_traits::{ProcessMessage, Poll, RouteMessageHandle};
use ockam_message::message::{Message, Route, RouterAddress, MessageType};
use alloc::string::String;
use core::ops::Deref;
use libc_print::*;

pub struct TestWorker {
    address: String,
    text: String,
    count: usize,
}

impl TestWorker {
    pub fn new(address: String, text: String) -> Self {
        TestWorker { address, text, count: 0 }
    }
}

impl Poll for TestWorker {
    fn poll(&mut self, q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
        libc_println!("{} is polling", self.text);
        let msg_text = "sent to you by TestWorker".as_bytes();
        let mut onward_addresses = Vec::new();

        onward_addresses.push(RouterAddress::worker_router_address_from_str("aabbccdd".into()).unwrap());
        let mut return_addresses = Vec::new();
        return_addresses.push(RouterAddress::worker_router_address_from_str(&self.address).unwrap());
        let m = Message {
            onward_route: Route { addresses: onward_addresses },
            return_route: Route { addresses: return_addresses },
            message_type: MessageType::Payload,
            message_body: msg_text.to_vec(),
        };
        let mut q = q_ref.deref().borrow_mut();
        q.route_message(m)?;
        Ok(true)
    }
}

impl ProcessMessage for TestWorker {
    fn handle_message(&mut self, message: Message, _q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
        libc_println!("TestWorker: {}", std::str::from_utf8(&message.message_body).unwrap());
        self.count += 1;
        if self.count > 3 { return Ok(false); }
        Ok(true)
    }
}

#[test]
fn test_node() {
    // create node
    let mut node = Node::new().unwrap();
    // Now create the worker(s) and register them with the worker manager
    let test_worker =
        Rc::new(RefCell::new(TestWorker::new("aabbccdd".into(), "text".into())));

    node.register_worker("aabbccdd".into(), Some(test_worker.clone()), Some(test_worker.clone())).expect("failed to register worker");

    if let Err(_s) = node.run() {
        assert!(false);
    } else {
        assert!(true);
    }
}
