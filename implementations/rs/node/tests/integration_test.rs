#![allow(unused)]
use ockam_node::Node;
extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::*;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam::message::{hex_vec_from_str, Address, Message, MessageType, Route, RouterAddress};
use ockam_no_std_traits::{EnqueueMessage, Poll, ProcessMessage};
use std::net::SocketAddr;
use std::str::FromStr;
use std::thread;

pub struct TestWorker {
    address: String,
    text: String,
    count: usize,
}

impl TestWorker {
    pub fn new(address: String, text: String) -> Self {
        TestWorker {
            address,
            text,
            count: 0,
        }
    }
}

impl Poll for TestWorker {
    fn poll(
        &mut self,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        println!("{} is polling", self.text);
        let msg_text = "sent to you by TestWorker".as_bytes();
        let mut onward_addresses = Vec::new();

        onward_addresses
            .push(RouterAddress::worker_router_address_from_str("aabbccdd".into()).unwrap());
        let mut return_addresses = Vec::new();
        return_addresses
            .push(RouterAddress::worker_router_address_from_str(&self.address).unwrap());
        let m = Message {
            onward_route: Route {
                addresses: onward_addresses,
            },
            return_route: Route {
                addresses: return_addresses,
            },
            message_type: MessageType::Payload,
            message_body: msg_text.to_vec(),
        };
        let mut q = enqueue_message_ref.deref().borrow_mut();
        q.enqueue_message(m)?;
        Ok(true)
    }
}

impl ProcessMessage for TestWorker {
    fn process_message(
        &mut self,
        message: Message,
        _q_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        self.count += 1;
        if self.count > 3 {
            return Ok(false);
        }
        Ok(true)
    }
}

#[test]
fn test_node() {
    // create node
    let mut node = Node::new("").unwrap();
    // Now create the worker(s) and register them with the worker manager
    let test_worker = Rc::new(RefCell::new(TestWorker::new(
        "aabbccdd".into(),
        "text".into(),
    )));

    node.register_worker(
        "aabbccdd".into(),
        Some(test_worker.clone()),
        Some(test_worker.clone()),
    )
    .expect("failed to register worker");

    if let Err(_s) = node.run() {
        assert!(false);
    } else {
        assert!(true);
    }
}

pub struct TestTcpWorker {
    is_initiator: bool,
    count: usize,
    remote: Address,
    local_address: Vec<u8>,
}

impl TestTcpWorker {
    pub fn new(is_initiator: bool, local_address: Vec<u8>, opt_remote: Option<Address>) -> Self {
        if let Some(r) = opt_remote {
            TestTcpWorker {
                is_initiator,
                count: 0,
                remote: r,
                local_address,
            }
        } else {
            TestTcpWorker {
                is_initiator,
                count: 0,
                remote: Address::TcpAddress(SocketAddr::from_str("127.0.0.1:4050").unwrap()),
                local_address,
            }
        }
    }
}

impl Poll for TestTcpWorker {
    fn poll(
        &mut self,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        if self.count == 0 && self.is_initiator {
            let mut route = Route {
                addresses: vec![
                    RouterAddress::tcp_router_address_from_str("127.0.0.1:4052").unwrap(),
                    RouterAddress::worker_router_address_from_str("00112233").unwrap(),
                ],
            };
            let addr = Address::WorkerAddress(self.local_address.clone());
            let m = Message {
                onward_route: route,
                return_route: Route {
                    addresses: vec![RouterAddress::from_address(addr).unwrap()],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            let mut q = enqueue_message_ref.deref().borrow_mut();
            q.enqueue_message(m)?;
        }
        self.count += 1;
        Ok(true)
    }
}

impl ProcessMessage for TestTcpWorker {
    fn process_message(
        &mut self,
        message: Message,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        if self.is_initiator {
            println!(
                "Initiator: message received: {}",
                String::from_utf8(message.message_body).unwrap()
            );
        } else {
            println!(
                "Responder: message received: {}",
                String::from_utf8(message.message_body).unwrap()
            );
        }
        if self.count < 5 {
            let addr = Address::WorkerAddress(self.local_address.clone());
            let m = Message {
                onward_route: message.return_route.clone(),
                return_route: Route {
                    addresses: vec![RouterAddress::from_address(addr).unwrap()],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            {
                let mut q = enqueue_message_ref.clone(); //rb
                let mut q = q.deref().borrow_mut();
                q.enqueue_message(m);
            }
            self.count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn responder_thread() {
    // create node
    let mut node = Node::new("responder").unwrap();

    // this is responder thread so needs a listen address.
    let listen_addr = "127.0.0.1:4052";
    node.initialize_transport(Some(listen_addr));

    // create test worker and register
    let worker_address = hex_vec_from_str("00112233".into()).unwrap();
    let worker = TestTcpWorker::new(false, worker_address.clone(), None);
    let worker_ref = Rc::new(RefCell::new(worker));
    node.register_worker("00112233".to_string(), Some(worker_ref.clone()), None);

    node.run();
}

pub fn initiator_thread() {
    // give the responder time to spin up
    thread::sleep(time::Duration::from_millis(1000));

    // create node
    let mut node = Node::new("initiator").unwrap();

    // get transport going. no listen address since this is initiator thread.
    node.initialize_transport(None)
        .expect("initialize transport failed");
    println!("created tcp_manager");

    // create test worker and register
    let worker_address = hex_vec_from_str("aabbccdd".into()).unwrap();
    let worker = TestTcpWorker::new(
        true,
        worker_address,
        Some(Address::TcpAddress(
            SocketAddr::from_str("127.0.0.1:4052").unwrap(),
        )),
    );
    let worker_ref = Rc::new(RefCell::new(worker));
    node.register_worker(
        "aabbccdd".to_string(),
        Some(worker_ref.clone()),
        Some(worker_ref.clone()),
    );

    node.run();
    return;
}

#[test]
fn test_tcp() {
    // spin up responder (listener) and initiator (client) threads
    let responder_handle = thread::spawn(|| responder_thread());
    let initiator_handle = thread::spawn(|| initiator_thread());
    match initiator_handle.join() {
        Ok(()) => {
            println!("initiator joined");
        }
        Err(_) => {
            assert!(false);
        }
    }
}
