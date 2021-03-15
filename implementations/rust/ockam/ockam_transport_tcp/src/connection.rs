use crate::error::TransportError;
use ockam::{Address, Context, Worker};
use ockam_core::async_trait::async_trait;
use ockam_core::Result;
use ockam_router::message::{RouterAddress, TransportMessage};
use ockam_router::{RouteTransportMessage, RouterError, ROUTER_ADDRESS, ROUTER_ADDRESS_TYPE_TCP};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::io;
use tokio::net::TcpStream;

// ToDo - revisit these values
const MAX_MESSAGE_SIZE: usize = 2048;
const DEFAULT_TCP_ADDRESS: &str = "127.0.0.1:4050";

pub struct TcpConnection {
    remote_address: std::net::SocketAddr,
    local_address: std::net::SocketAddr,
    _blocking: bool,
    message_buff: Vec<u8>,
    message_length: usize,
    stream: Option<tokio::net::TcpStream>,
}

impl TcpConnection {
    pub fn create(remote_address: SocketAddr) -> Box<TcpConnection> {
        let c = Box::new(TcpConnection {
            remote_address,
            local_address: SocketAddr::from_str(&DEFAULT_TCP_ADDRESS).unwrap(),
            _blocking: true,
            message_buff: vec![],
            message_length: 0,
            stream: None,
        });
        c
    }

    pub async fn new_from_stream(stream: TcpStream) -> Result<Box<TcpConnection>> {
        match stream.peer_addr() {
            Ok(peer) => {
                let c = Box::new(TcpConnection {
                    remote_address: peer,
                    local_address: SocketAddr::from_str(&DEFAULT_TCP_ADDRESS).unwrap(),
                    _blocking: true,
                    message_buff: vec![],
                    message_length: 0,
                    stream: Some(stream),
                });
                Ok(c)
            }
            Err(_) => Err(TransportError::PeerNotFound.into()),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        match self.stream {
            Some(_) => Ok(()), //Err(TransportError::AlreadyConnected.into()),
            None => match TcpStream::connect(&self.remote_address).await {
                Ok(s) => {
                    self.stream = Some(s);
                    Ok(())
                }
                Err(_) => Err(TransportError::ConnectFailed.into()),
            },
        }
    }

    pub async fn send(&mut self, buff: &[u8]) -> Result<usize> {
        let mut i = 0;
        return if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.writable().await) {
                    return Err(TransportError::CheckConnection.into());
                }
                match stream.try_write(&buff[i..]) {
                    Ok(n) if n == buff.len() => {
                        return Ok(n);
                    }
                    Ok(n) => {
                        i += n;
                        continue;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        return Err(TransportError::CheckConnection.into());
                    }
                    Err(_) => {
                        return Err(TransportError::CheckConnection.into());
                    }
                }
            }
        } else {
            Err(TransportError::NotConnected.into())
        };
    }

    pub async fn receive(&mut self, buff: &mut [u8]) -> Result<usize> {
        if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.readable().await) {
                    return Err(TransportError::CheckConnection.into());
                }
                match stream.try_read(buff) {
                    Ok(n) => {
                        return if 0 == n {
                            Err(TransportError::ConnectionClosed.into())
                        } else {
                            Ok(n)
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    _ => {
                        return Err(TransportError::ReceiveFailed.into());
                    }
                }
            }
        } else {
            Err(TransportError::CheckConnection.into())
        }
    }

    pub async fn send_message(&mut self, mut msg: TransportMessage) -> Result<usize> {
        if !msg.onward_route.addrs.is_empty() {
            if msg.onward_route.addrs[0] == self.get_remote_address() {
                msg.onward_route.addrs.remove(0);
            }
        }
        msg.return_route
            .addrs
            .insert(0, self.get_local_address().clone());
        return match serde_bare::to_vec::<TransportMessage>(&msg) {
            Ok(mut msg_vec) => {
                if msg_vec.len() > MAX_MESSAGE_SIZE - 2 {
                    return Err(TransportError::IllFormedMessage.into());
                }
                let len = msg_vec.len() as u16;
                let mut msg_len_vec = len.to_be_bytes().to_vec();
                msg_len_vec.append(&mut msg_vec);
                return self.send(&msg_len_vec).await;
            }
            Err(_) => Err(TransportError::IllFormedMessage.into()),
        };
    }

    pub async fn receive_message(&mut self) -> Result<TransportMessage> {
        loop {
            let mut recv_buff = [0u8; MAX_MESSAGE_SIZE];

            // first see if we have a complete message from the last call
            // if not, read additional bytes
            if self.message_buff.len() <= self.message_length as usize {
                let bytes_received = self.receive(&mut recv_buff).await?;
                self.message_buff
                    .append(&mut recv_buff[0..bytes_received].to_vec());
            }

            if self.message_length == 0 {
                let (len, _) = recv_buff.split_at(2);
                self.message_length = u16::from_be_bytes(len.try_into().unwrap()) as usize;
                self.message_buff.remove(0);
                self.message_buff.remove(0);
            }

            if self.message_length as usize <= self.message_buff.len() {
                // we have a complete message
                return match serde_bare::from_slice::<TransportMessage>(&self.message_buff) {
                    Ok(mut m) => {
                        // scoot any remaining bytes to the beginning of the buffer
                        for i in 0..self.message_buff.len() - self.message_length {
                            self.message_buff[i] = self.message_buff[i + self.message_length];
                        }
                        self.message_buff
                            .truncate(self.message_buff.len() - self.message_length);
                        self.message_length = 0;
                        if self.message_buff.len() >= 2 {
                            let len = &self.message_buff[..2];
                            self.message_length =
                                u16::from_be_bytes(len.try_into().unwrap()) as usize;
                            self.message_buff.remove(0);
                            self.message_buff.remove(0);
                        }

                        // if first address in onward route is ours, remove it
                        if !m.onward_route.addrs.is_empty()
                            && m.onward_route.addrs[0] == self.get_local_address()
                        {
                            m.onward_route.addrs.remove(0);
                        }

                        if !m.onward_route.addrs.is_empty()
                            && m.onward_route.addrs[0].address_type == ROUTER_ADDRESS_TYPE_TCP
                        {
                            let router_addr =
                                serde_bare::to_vec::<SocketAddr>(&self.local_address).unwrap();
                            m.return_route.addrs.push(RouterAddress {
                                address_type: ROUTER_ADDRESS_TYPE_TCP,
                                address: router_addr,
                            });
                            self.send_message(m).await?;
                            continue;
                        }
                        Ok(m)
                    }
                    Err(_) => Err(TransportError::IllFormedMessage.into()),
                };
            }
        }
    }

    pub fn get_local_address(&self) -> RouterAddress {
        return match &self.stream {
            Some(stream) => {
                let ra = serde_bare::to_vec::<SocketAddr>(&stream.local_addr().unwrap()).unwrap();
                RouterAddress {
                    address_type: ROUTER_ADDRESS_TYPE_TCP,
                    address: ra,
                }
            }
            None => RouterAddress {
                address_type: ROUTER_ADDRESS_TYPE_TCP,
                address: vec![],
            },
        };
    }

    pub fn get_remote_address(&self) -> RouterAddress {
        let ra = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_TCP,
            address: ra,
        }
    }

    pub fn get_worker_address(&self) -> Address {
        let addr = self.get_remote_address();
        Address::from(serde_bare::to_vec::<RouterAddress>(&addr).unwrap())
    }

    pub fn get_routeable_address(&self) -> Vec<u8> {
        let mut v = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        v.insert(0, ROUTER_ADDRESS_TYPE_TCP);
        v
    }

    pub fn get_router_address(&self) -> RouterAddress {
        let v = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        RouterAddress {
            address_type: 1,
            address: v,
        }
    }

    pub fn get_remote_socket(&self) -> SocketAddr {
        return match &self.stream {
            Some(s) => s.peer_addr().unwrap(),
            None => SocketAddr::from_str("0.0.0.0:00").unwrap(),
        };
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum TcpWorkerMessage {
    SendMessage(TransportMessage),
    Receive,
}

#[async_trait]
impl Worker for Box<TcpConnection> {
    type Message = TcpWorkerMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            TcpWorkerMessage::SendMessage(m) => {
                if self.send_message(m).await.is_err() {
                    Err(TransportError::ConnectionClosed.into())
                } else {
                    Ok(())
                }
            }
            TcpWorkerMessage::Receive => {
                return if let Ok(msg) = self.receive_message().await {
                    if msg.onward_route.addrs.is_empty() {
                        return Err(RouterError::NoRoute.into());
                    }
                    return match ctx
                        .send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(msg))
                        .await
                    {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    };
                } else {
                    Ok(())
                };
            }
        };
    }
}
