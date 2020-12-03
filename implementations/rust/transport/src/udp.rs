#[allow(unused)]
use ockam_message::message::*;
use ockam_message::MAX_MESSAGE_SIZE;
use ockam_system::commands::RouterCommand::ReceiveMessage;
use ockam_system::commands::{OckamCommand, RouterCommand, TransportCommand};
use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;

pub struct UdpTransport {
    socket: UdpSocket,
    rx: std::sync::mpsc::Receiver<OckamCommand>,
    _tx: std::sync::mpsc::Sender<OckamCommand>,
    router_tx: std::sync::mpsc::Sender<OckamCommand>,
}

impl UdpTransport {
    pub fn new(
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        local_udp_socket: SocketAddr,
    ) -> Result<UdpTransport, String> {
        // Try to create socket at given address
        match UdpSocket::bind(local_udp_socket) {
            Ok(socket) => {
                socket.set_nonblocking(true).unwrap();
                // Register address type with Router
                router_tx
                    .send(OckamCommand::Router(RouterCommand::Register(
                        AddressType::Udp,
                        tx.clone(),
                    )))
                    .unwrap();

                Ok(UdpTransport {
                    socket,
                    rx,
                    _tx: tx,
                    router_tx,
                })
            }
            Err(_unused) => {
                println!("failed to create socket");
                Err("failed to create socket".to_string())
            }
        }
    }

    pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
        let remote_address = m.onward_route.addresses.remove(0);

        match self.socket.local_addr() {
            Ok(la) => match RouterAddress::from_address(Address::UdpAddress(la)) {
                Some(ra) => {
                    m.return_route.addresses.insert(0, ra);
                    let mut v = vec![];
                    Message::encode(&m, &mut v)?;
                    match self
                        .socket
                        .send_to(v.as_slice(), remote_address.address.as_string())
                    {
                        Ok(_) => Ok(()),
                        Err(s) => {
                            println!("send_message failed {}", s.to_string());
                            Err("send_message error".to_string())
                        }
                    }
                }
                None => Err("send_message error".to_string()),
            },
            Err(_unused) => Err("send_message".to_string()),
        }
    }

    pub fn receive_message(&mut self) -> Result<bool, String> {
        let mut buff = [0; MAX_MESSAGE_SIZE];
        match self.socket.recv_from(&mut buff) {
            Ok((s, _)) => match Message::decode(&buff[0..s]) {
                Ok((m, _unused)) => {
                    if !m.onward_route.addresses.is_empty()
                        && ((m.onward_route.addresses[0].a_type == AddressType::Udp)
                            || (m.onward_route.addresses[0].a_type == AddressType::Tcp))
                    {
                        match self.send_message(m) {
                            Err(s) => Err(s),
                            Ok(()) => Ok(true),
                        }
                    } else {
                        match self.router_tx.send(OckamCommand::Router(ReceiveMessage(m))) {
                            Ok(_unused) => Ok(true),
                            Err(_) => Err("send to router failed".to_string()),
                        }
                    }
                }
                _ => Err("decode failed".to_string()),
            },
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(false),
                _ => Err("socket receive failed".to_string()),
            },
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut got: bool = true;
        let mut keep_going = true;

        while got && keep_going {
            match self.receive_message() {
                Ok(b) => {
                    got = b;
                }
                Err(_) => {
                    keep_going = false;
                }
            }
        }

        got = true;
        while got && keep_going {
            got = false;
            if let Ok(tc) = self.rx.try_recv() {
                match tc {
                    OckamCommand::Transport(TransportCommand::SendMessage(m)) => {
                        if let Err(s) = self.send_message(m) {
                            println!("udp send_message failed: {}", s);
                            return false;
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
        }
        keep_going
    }
}
