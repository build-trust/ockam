#[allow(unused)]
use ockam_message::message::*;
use ockam_system::commands::RouterCommand::ReceiveMessage;
use ockam_system::commands::{OckamCommand, RouterCommand, TransportCommand};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{SocketAddr, TcpListener};
use std::str;
use std::str::FromStr;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{io, thread, time};

pub struct TcpManager {
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    listen_addr: Option<SocketAddr>,
    timeout: Duration,
    listener: Option<TcpListener>,
    connections: HashMap<String, TcpTransport>,
    addresses: Vec<String>,
}

impl TcpManager {
    pub fn connect(&mut self, address: SocketAddr) -> Result<Address, String> {
        let stream = TcpStream::connect(address);
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(true);
                self.add_connection(stream);
                Ok(Address::TcpAddress(address))
            }
            Err(e) => Err("tcp failed to connect".into()),
        }
    }

    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        listen_addr: Option<SocketAddr>,
        tmo: Option<Duration>,
    ) -> Result<TcpManager, String> {
        router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Tcp,
            tx.clone(),
        )));
        let connections = HashMap::new();

        let mut timeout: Duration;
        if let Some(t) = tmo {
            timeout = t;
        } else {
            timeout = Duration::new(5, 0);
        }

        let mut listener: Option<TcpListener> = None;
        return match listen_addr {
            Some(la) => {
                if let Ok(l) = TcpListener::bind(la) {
                    l.set_nonblocking(true);
                    Ok(TcpManager {
                        rx,
                        tx,
                        router_tx,
                        listen_addr,
                        timeout,
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
                tx,
                router_tx,
                listen_addr,
                timeout,
                listener: None,
                connections,
                addresses: vec![],
            }),
        };
    }

    fn add_connection(&mut self, stream: TcpStream) -> bool {
        let peer_addr = stream.peer_addr().unwrap().clone();
        let local_addr = stream.local_addr().unwrap().clone();
        let tcp_xport =
            TcpTransport::new(self.timeout.clone(), stream, self.router_tx.clone()).unwrap();
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
                            println!("got connection");
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
                    OckamCommand::Transport((TransportCommand::SendMessage(mut m))) => {
                        let addr = m.onward_route.addresses.get_mut(0).unwrap();
                        let addr = addr.address.as_string();
                        println!("tcp manager, getting connection {}", &addr);
                        if let Some(tcp_xport) = self.connections.get_mut(&addr) {
                            match tcp_xport.send_message(m) {
                                Err(e) => {
                                    println!("connection not found");
                                    keep_going = false;
                                }
                                _ => {}
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
                match self.connections.get_mut(&a.to_string()) {
                    Some(t) => {
                        t.receive_message();
                    }
                    None => {}
                }
            }
        }

        keep_going
    }
}

pub struct TcpTransport {
    timeout: Duration,
    stream: TcpStream,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
    local_address: Address,
}

impl TcpTransport {
    pub fn new(
        timeout: Duration,
        stream: TcpStream,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
    ) -> Result<TcpTransport, String> {
        let local_address = Address::TcpAddress(stream.local_addr().unwrap());
        Ok(TcpTransport {
            timeout,
            stream,
            router_tx,
            local_address,
        })
    }

    pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
        let remote_address = m.onward_route.addresses.remove(0);
        m.return_route.addresses.insert(
            0,
            RouterAddress::from_address(self.local_address.clone()).unwrap(),
        );
        let mut v = vec![];
        println!("\nsending onward:");
        println!("message type: {:?}", &m.message_type);
        println!("sending to {}", remote_address.address.as_string());
        m.onward_route.print_route();
        println!("sending return:");
        m.return_route.print_route();
        Message::encode(&m, &mut v);
        return match self.stream.write(v.as_slice()) {
            Ok(n) => Ok(()),
            Err(e) => Err("tcp write failed".into()),
        };
    }

    pub fn receive_message(&mut self) -> Result<bool, String> {
        let mut buff = [0u8; 16348];
        match self.stream.read(&mut buff) {
            Ok(len) => {
                match Message::decode(&buff[0..len]) {
                    Ok((mut m, _)) => {
                        println!("\nreceiving onward:");
                        println!("received from: {:?}", self.stream.peer_addr());
                        m.onward_route.print_route();
                        println!("message type: {:?}", m.message_type);
                        println!("receiving return:");
                        m.return_route.print_route();
                        if !m.onward_route.addresses.is_empty()
                            && ((m.onward_route.addresses[0].a_type == AddressType::Udp)
                                || (m.onward_route.addresses[0].a_type == AddressType::Tcp))
                        {
                            match self.send_message(m) {
                                Err(s) => {
                                    return Err(s);
                                }
                                Ok(()) => {
                                    return Ok(true);
                                }
                            }
                        } else {
                            match self.router_tx.send(OckamCommand::Router(ReceiveMessage(m))) {
                                Ok(_unused) => {
                                    return Ok(true);
                                }
                                Err(s) => {
                                    return Err("send to router failed".to_string());
                                }
                            }
                        }
                    }
                    _ => {
                        return Err("decode failed".to_string());
                    }
                }
                Ok(true)
            }
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(false),
                _ => Err("tcp receive failed".to_string()),
            },
        }
    }
}
