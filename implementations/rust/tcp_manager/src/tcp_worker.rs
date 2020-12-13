#![allow(unused)]
extern crate alloc;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::Deref;
use ockam_message::message::{Address, AddressType, Codec, Message, RouterAddress, varint_size};
use ockam_no_std_traits::{RouteMessage, ProcessMessage, RouteMessageHandle, Poll};
use std::net::TcpStream;
use ockam_message::MAX_MESSAGE_SIZE;
use std::io::{Write, Read};
use std::io;

pub struct TcpWorker {
    stream: TcpStream,
    message: [u8; MAX_MESSAGE_SIZE],
    offset: usize,
    message_length: usize,
}

impl ProcessMessage for TcpWorker {
    fn process_message(&mut self, mut message: Message, queue: RouteMessageHandle<Message>) -> Result<bool, String> {
        message.onward_route.addresses.remove(0);
        let local_address = Address::TcpAddress(self.stream.local_addr().unwrap());
        message.return_route
            .addresses
            .insert(0, RouterAddress::from_address(local_address).unwrap());
        let mut v = vec![];
        Message::encode(&message, &mut v)?;

        // encode the message length and write it as the first byte (or 2)
        let mut msg_len: Vec<u8> = vec![];
        u16::encode(&(v.len() as u16), &mut msg_len);
        self.stream
            .write(msg_len.as_slice())
            .expect("tcp write failed");
        return match self.stream.write(v.as_slice()) {
            Ok(_) => Ok(true),
            Err(_) => Err("tcp write failed".into()),
        };
    }
}

impl Poll for TcpWorker {
    fn poll(&mut self, message_router_handle: RouteMessageHandle<Message>) -> Result<bool, String> {
        self.stream.set_nonblocking(true);
        let mut tcp_buff: [u8; MAX_MESSAGE_SIZE] = [0u8; MAX_MESSAGE_SIZE];
        match self.stream.read(&mut tcp_buff[0..]) {
            Ok(mut tcp_len) => {
                if tcp_len == 0 {
                    return Ok(false);
                }

                let mut tcp_vec = tcp_buff[0..tcp_len].to_vec();
                while tcp_vec.len() > 0 {
                    // if self.message_length is 0, then decode the next byte(s) as message length
                    if self.message_length == 0 {
                        self.set_msg_len(&mut tcp_vec)?;
                    }

                    // we have a message length and an offset into the message buffer,
                    // try to read enough bytes to fill the message
                    let mut remaining_msg_bytes = self.message_length - self.offset;

                    if tcp_vec.len() < remaining_msg_bytes {
                        // not enough bytes to complete message, copy what there is and return
                        self.message[self.offset..(self.offset + tcp_vec.len())]
                            .clone_from_slice(&tcp_vec);
                        self.offset += tcp_vec.len();
                        return Ok(false);
                    }

                    // we have a complete message, route it
                    let bytes_to_clone = self.message_length - self.offset;
                    self.message[self.offset..self.message_length]
                        .clone_from_slice(&tcp_vec[0..bytes_to_clone]);
                    tcp_vec = tcp_vec.split_off(bytes_to_clone);
                    self.decode_and_route_message(message_router_handle.clone())?;
                    self.offset = 0;
                    self.message_length = 0;
                }
                Ok(true)
            }
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(true),
                _ => Err("***tcp receive failed".to_string()),
            },
        }

    }
}

impl TcpWorker {
    pub fn new_connection(stream: TcpStream) -> Self {
        TcpWorker {
            stream,
            message: [0u8;MAX_MESSAGE_SIZE],
            offset: 0,
            message_length: 0
        }
    }

    fn set_msg_len(&mut self, varint: &mut Vec<u8>) -> Result<(), String> {
        if let Ok((l, b)) = u16::decode(varint) {
            self.message_length = l as usize;
            varint.remove(0);
            if varint_size(l) == 2 {
                varint.remove(0);
            }
            Ok(())
        } else {
            Err("seg_msg_len failed".into())
        }
    }

    fn decode_and_route_message(&mut self, message_router_handle: RouteMessageHandle<Message>) -> Result<bool, String> {
        match Message::decode(&self.message[0..self.message_length]) {
            Ok((mut m_decoded, _)) => {
                // fix up return tcp address with nat-ed address
                let tcp_return = Address::TcpAddress(self.stream.peer_addr().unwrap());
                m_decoded.return_route.addresses[0] =
                    RouterAddress::from_address(tcp_return).unwrap();
                if !m_decoded.onward_route.addresses.is_empty()
                    && ((m_decoded.onward_route.addresses[0].a_type == AddressType::Udp)
                    || (m_decoded.onward_route.addresses[0].a_type == AddressType::Tcp))
                {
                    self.send_message(m_decoded);
                    Ok(true)
                } else {
                    let r = message_router_handle.clone();
                    let mut r = r.deref().borrow_mut();
                    r.route_message(m_decoded);
                    Ok(true)
                }
            }
            Err(_) => {
                Err("message decode failed".into())
            }
        }
    }


    fn send_message(&mut self, mut m: Message) -> Result<(), String> {
        m.onward_route.addresses.remove(0);
        let local_address = Address::TcpAddress(self.stream.local_addr().unwrap());
        m.return_route
            .addresses
            .insert(0, RouterAddress::from_address(local_address).unwrap());
        let mut v = vec![];
        Message::encode(&m, &mut v)?;

        // encode the message length and write it as the first byte (or 2)
        let mut mlen: Vec<u8> = vec![];
        u16::encode(&(v.len() as u16), &mut mlen);
        self.stream
            .write(mlen.as_slice())
            .expect("tcp write failed");
        return match self.stream.write(v.as_slice()) {
            Ok(_) => Ok(()),
            Err(_) => Err("tcp write failed".into()),
        };
    }

}