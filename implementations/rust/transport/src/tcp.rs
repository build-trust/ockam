use futures::io::Error;
#[allow(unused)]
use ockam_message::message::*;
use ockam_message::MAX_MESSAGE_SIZE;
use ockam_system::commands::RouterCommand::ReceiveMessage;
use ockam_system::commands::{OckamCommand, RouterCommand, TransportCommand};
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;

pub struct TcpManager {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    _timeout: Duration,
    listener: Option<TcpListener>,
    connections: HashMap<String, TcpTransport>,
    addresses: Vec<String>,
}

impl TcpManager {
    pub fn connect(&mut self, address: SocketAddr) -> Result<Address, String> {
        let stream = TcpStream::connect(address);
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(true).unwrap();
                self.add_connection(stream);
                Ok(Address::TcpAddress(address))
            }
            Err(e) => Err(format!("tcp failed to connect: {}", e)),
        }
    }

    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        listen_addr: Option<SocketAddr>,
        tmo: Option<Duration>,
    ) -> Result<TcpManager, String> {
        router_tx
            .send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Tcp,
                tx.clone(),
            )))
            .unwrap();
        let connections = HashMap::new();

        let timeout = tmo.unwrap_or(Duration::new(5, 0));

        return match listen_addr {
            Some(la) => {
                if let Ok(l) = TcpListener::bind(la) {
                    l.set_nonblocking(true).unwrap();
                    Ok(TcpManager {
                        rx,
                        _tx: tx,
                        router_tx,
                        _timeout: timeout,
                        listener: Some(l),
                        connections,
                        addresses: vec![],
                    })
                } else {
                    Err("failed to bind tcp listener".into())
                }
            }
            _ => Ok(TcpManager {
                rx,
                _tx: tx,
                router_tx,
                _timeout: timeout,
                listener: None,
                connections,
                addresses: vec![],
            }),
        };
    }

    fn add_connection(&mut self, stream: TcpStream) -> bool {
        stream.set_nonblocking(true).unwrap();
        let peer_addr = stream.peer_addr().unwrap().clone();
        let tcp_xport = TcpTransport::new(stream, self.router_tx.clone()).unwrap();
        self.connections.insert(peer_addr.to_string(), tcp_xport);
        self.addresses.push(peer_addr.to_string());
        true
    }

    pub fn poll(&mut self) -> bool {
        let mut got: bool = true;
        let mut keep_going = true;

        while got && keep_going {
            // listen for connect
            got = false;
            if let Some(listener) = &self.listener {
                for s in listener.incoming() {
                    match s {
                        Ok(stream) => {
                            keep_going = self.add_connection(stream);
                            break;
                        }
                        Err(e) => match e.kind() {
                            io::ErrorKind::WouldBlock => {
                                break;
                            }
                            _ => {
                                println!("tcp listen error");
                                keep_going = false;
                                break;
                            }
                        },
                    }
                }
            }

            if let Ok(tc) = self.rx.try_recv() {
                match tc {
                    OckamCommand::Transport(TransportCommand::SendMessage(mut m)) => {
                        let addr = m.onward_route.addresses.get_mut(0).unwrap();
                        let addr = addr.address.as_string();
                        if let Some(tcp_xport) = self.connections.get_mut(&addr) {
                            match tcp_xport.send_message(m) {
                                Err(e) => {
                                    println!("send_message failed: {}", e);
                                    keep_going = false;
                                }
                                _ => {}
                            }
                        } else {
                            println!("can't find connection {}", addr);
                            println!("{} connections in hashmap", self.connections.len());
                            for (c, t) in &self.connections {
                                println!("{}", c);
                            }
                        }
                    }
                    OckamCommand::Transport(TransportCommand::Stop) => {
                        keep_going = false;
                        break;
                    }
                    _ => {
                        println!("unrecognized command");
                    }
                }
            } // end match rx.try_recv()

            // check for receives
            for a in &self.addresses {
                let mut l = 0;
                match self.connections.get_mut(&a.to_string()) {
                    Some(t) => {
                        loop {
                            l += 1;
                            match t.try_receive() {
                                Ok(b) => {
                                    if !b {
                                        break;
                                    }
                                }
                                Err(s) => {
                                    println!("tcp read failed: {}", s);
                                    return false;
                                }
                            }
                        }
                        if l > 2 {
                            println!("looped {} times", l);
                        }
                    }
                    None => {}
                }
            }
        }

        keep_going
    }
}

pub struct TcpTransport {
    stream: TcpStream,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    message: [u8; MAX_MESSAGE_SIZE],
    offset: usize,
    message_length: usize,
}

impl TcpTransport {
    pub fn new(
        stream: TcpStream,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Result<TcpTransport, String> {
        let local_address = Address::TcpAddress(stream.local_addr().unwrap());
        Ok(TcpTransport {
            stream,
            router_tx,
            message: [0u8; MAX_MESSAGE_SIZE],
            offset: 0,
            message_length: 0,
        })
    }

    pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
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

    fn route_message(&mut self) -> Result<(), String> {
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
                    self.send_message(m_decoded)
                } else {
                    self.router_tx
                        .send(OckamCommand::Router(ReceiveMessage(m_decoded)))
                        .expect("send to router failed");
                    Ok(())
                }
            }
            Err(_) => {
                return Err("message decode failed".into());
            }
        }
    }

    pub fn try_receive(&mut self) -> Result<bool, String> {
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
                    self.route_message()?;
                    self.offset = 0;
                    self.message_length = 0;
                }
                Ok(true)
            }
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(false),
                _ => Err("***tcp receive failed".to_string()),
            },
        }
    }
}
