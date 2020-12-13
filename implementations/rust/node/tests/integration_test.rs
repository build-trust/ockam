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
use std::thread;
use ockam_tcp_manager::tcp_manager::TcpManager;
use core::time;

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
    fn process_message(&mut self, message: Message, _q_ref: RouteMessageHandle<Message>) -> Result<bool, String> {
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

pub struct TestTcpWorker {

}

pub fn responder_thread() {
    println!("responding");
    // create node
    let mut node = Node::new().unwrap();

    let listen_addr = "127.0.0.1:4052";
    if let Ok(tcp_manager) = TcpManager::new(Some("127.0.0.1:4052")) {
        println!("created tcp_manager");
        thread::sleep(time::Duration::from_millis(1000));
    } else {
        return;
    }
}

pub fn initiator_thread() {
    println!("initiating");
    thread::sleep(time::Duration::from_micros(500));
    // create node
    let mut node = Node::new().unwrap();

    if let Ok(mut tcp_manager) = TcpManager::new(None) {
        println!("created tcp_manager");
        let try_connect = tcp_manager.try_connect("127.0.0.1:4052");
        match try_connect {
            Ok(()) => {},
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        }
    } else {
        assert!(false);
    }
}

#[test]
fn test_tcp() {
    // spin up responder (listener) and initiator (client) threads
    let responder_handle = thread::spawn(|| responder_thread());
    let initiator_handle = thread::spawn(|| initiator_thread());
    initiator_handle.join();
    responder_handle.join();
}
