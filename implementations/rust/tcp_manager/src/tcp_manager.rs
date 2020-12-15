#![allow(unused)]

extern crate alloc;

use crate::tcp_worker::TcpWorker;
use alloc::rc::Rc;
use libc_print::*;
use ockam_message::message::{
    varint_size, Address, AddressType, Codec, Message, MessageType, Route, RouterAddress,
};
use ockam_message::MAX_MESSAGE_SIZE;
use ockam_no_std_traits::{EnqueueMessage, Poll, ProcessMessage};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::str::FromStr;

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
            None => Ok(TcpManager {
                connections,
                listener: None,
            }),
        };
    }

    fn accept_new_connections(&mut self) -> Result<bool, String> {
        let mut keep_going = true;
        if let Some(listener) = &self.listener {
            for s in listener.incoming() {
                match s {
                    Ok(stream) => {
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
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        // if we don't already have a connection for onward address, try to create one
        let address = &message.onward_route.addresses[0].address;
        if let None = self.connections.get_mut(&address.as_string()) {
            self.try_connect(&address.as_string());
        }
        if let Some(connection) = self.connections.get_mut(&address.as_string()) {
            connection.process_message(message, enqueue_message_ref)?;
        } else {
            // todo - kick message back with error
            libc_println!("failed to connect to {:?}", address);
        }
        Ok(true)
    }
}

impl Poll for TcpManager {
    fn poll(
        &mut self,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        if matches!(self.listener, Some(_)) {
            self.accept_new_connections()?;
        }
        for (_, mut tcp_worker) in self.connections.iter_mut() {
            tcp_worker.poll(enqueue_message_ref.clone())?;
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
