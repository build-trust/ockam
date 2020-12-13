#![allow(unused)]

extern crate alloc;

use alloc::rc::Rc;
use libc_print::*;
use ockam_message::message::{AddressType, Message, MessageType, Route, Address, RouterAddress, Codec, varint_size};
use ockam_no_std_traits::{RouteMessage, ProcessMessage, Poll, RouteMessageHandle};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::str::FromStr;
use ockam_message::MAX_MESSAGE_SIZE;
use std::io::{Read, Write};
use crate::tcp_worker::TcpWorker;

pub struct TcpManager {
    connections: HashMap<String, TcpWorker>,
    listener: Option<TcpListener>,
}

impl TcpManager {
    pub fn new(listen_addr: Option<&str>) -> Result<TcpManager, String> {
        let connections: HashMap<String, TcpWorker> = HashMap::new();
        return match listen_addr {
            Some(la) => {
                if let Ok(l) = TcpListener::bind(la) {
                    l.set_nonblocking(true).unwrap();
                    Ok(TcpManager {
                        connections,
                        listener: Some(l),
                    })
                } else {
                    Err("failed to bind tcp listener".into())
                }
            }
            None => {
                Ok(TcpManager {
                    connections,
                    listener: None,
                })
            }
        }
    }

    fn accept_new_connections(&mut self) -> Result<bool, String> {
        let mut keep_going = true;
        if let Some(listener) = &self.listener {
            for s in listener.incoming() {
                match s {
                    Ok(stream) => {
                        println!("accepted connection");
                        stream.set_nonblocking(true).unwrap();
                        let peer_addr = stream.peer_addr().unwrap().clone();
                        let tcp_worker = TcpWorker::new_connection(stream);
                        self.connections.insert(peer_addr.to_string(), tcp_worker);
                    }
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            break;
                        }
                        _ => {
                            println!("tcp listen error");
                            return Ok(false);
                        }
                    },
                }
            }
        }
        Ok(true)
    }

    pub fn try_connect(&mut self, address: &str) -> Result<(), String> {
        let stream = TcpStream::connect(address);
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(true).unwrap();
                let peer_addr = stream.peer_addr().unwrap().clone();
                let tcp_worker = TcpWorker::new_connection(stream);
                self.connections.insert(peer_addr.to_string(), tcp_worker);
                Ok(())
            }
            Err(e) => Err(format!("tcp failed to connect: {}", e)),
        }

    }

}

impl ProcessMessage for TcpManager {
    fn process_message(
        &mut self,
        message: Message,
        message_router_handle: RouteMessageHandle<Message>,
    ) -> Result<bool, String> {
        Ok(true)
    }
}

impl Poll for TcpManager {
    fn poll(&mut self, message_router_handle: RouteMessageHandle<Message>) -> Result<bool, String> {
        libc_println!("polling for tcpmanager");
        let m = Message {
            onward_route: Route { addresses: vec![] },
            return_route: Route { addresses: vec![] },
            message_type: MessageType::Payload,
            message_body: vec![],
        };
        let mut q = message_router_handle.deref().borrow_mut();
        q.route_message(m)?;
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
