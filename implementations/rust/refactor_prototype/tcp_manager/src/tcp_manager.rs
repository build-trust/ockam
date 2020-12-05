extern crate alloc;

use crate::tcp_worker::TcpTransport;
use alloc::rc::Rc;
use libc_print::*;
use ockam_message::message::{AddressType, Message, MessageType, Route};
use ockam_no_std_traits::{Enqueue, MessageHandler, Poll};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::str::FromStr;

pub struct TcpManager {
    connections: HashMap<String, Box<dyn MessageHandler>>,
    listener: Option<TcpListener>,
}

impl TcpManager {
    pub fn new(listen_addr: Option<&str>) -> Result<TcpManager, String> {
        let connections: HashMap<String, Box<dyn MessageHandler>> = HashMap::new();
        Ok(TcpManager {
            connections,
            listener: None,
        })
    }
}

impl MessageHandler for TcpManager {
    fn handle_message(
        &mut self,
        message: Message,
        q_ref: Rc<RefCell<dyn Enqueue<Message>>>,
    ) -> Result<bool, String> {
        libc_println!("routing address type tcp");
        Ok(true)
    }
}

impl Poll for TcpManager {
    fn poll(&mut self, q_ref: Rc<RefCell<dyn Enqueue<Message>>>) -> Result<bool, String> {
        libc_println!("polling for tcpmanager");
        let m = Message {
            onward_route: Route { addresses: vec![] },
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: vec![],
        };
        let mut q = q_ref.deref().borrow_mut();
        q.enqueue(m);
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
