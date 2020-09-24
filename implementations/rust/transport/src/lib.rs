use ockam_common::commands::ockam_commands::*;
use ockam_message::*;
use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, SocketAddrV4, UdpSocket},
    str::{self, FromStr},
    sync::mpsc::{Receiver, Sender},
};

pub struct UdpTransport {
    connections: HashMap<String, UdpConnection>,
    rx: Receiver<OckamCommand>,
    tx: Sender<OckamCommand>,
    router_tx: Sender<OckamCommand>,
}

impl UdpTransport {
    pub fn new(
        rx: Receiver<OckamCommand>,
        tx: Sender<OckamCommand>,
        router_tx: Sender<OckamCommand>,
    ) -> UdpTransport {
        let _ = router_tx.send(OckamCommand::Router(RouterCommand::Register(
            AddressType::Udp,
            tx.clone(),
        )));
        UdpTransport {
            connections: HashMap::new(),
            rx,
            tx,
            router_tx,
        }
    }

    pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
        let udp = match self
            .connections
            .get_mut(&m.onward_route.addresses[0].address.as_string())
        {
            Some(c) => c,
            None => {
                println!("connection not found");
                return Err("connection not found".to_string());
            }
        };
        m.onward_route.addresses.remove(0);
        let local_addr: SocketAddr;
        match udp.socket.local_addr() {
            Ok(la) => local_addr = la,
            Err(_) => return Err("send_message".to_string()),
        };

        if let Some(ra) = RouterAddress::from_address(Address::UdpAddress(local_addr)) {
            m.return_route.addresses.insert(0, ra);
        }
        let mut v = vec![];
        let _ = Message::encode(m, &mut v);
        match udp.socket.send(v.as_slice()) {
            Ok(_) => Ok(()),
            Err(_) => Err("socket.send error".to_string()),
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut got;
        let mut keep_going = true;
        for (_, c) in self.connections.iter_mut() {
            keep_going = c.receive_message(&mut self.router_tx, &mut self.tx);
        }

        got = true;
        while got {
            got = false;
            //               match self.rx.try_recv() {
            if let Ok(tc) = self.rx.try_recv() {
                match tc {
                    OckamCommand::Transport(TransportCommand::Add(local, remote)) => {
                        got = true;
                        println!("creating udp connection");
                        let c = match UdpConnection::new(&local, &remote) {
                            Ok(c) => c,
                            Err(_) => return false,
                        };
                        println!("Added {} to transport", remote);
                        self.connections.insert(c.address_as_string(), c);
                    }
                    OckamCommand::Transport(TransportCommand::SendMessage(m)) => {
                        let _ = self.send_message(m);
                    }
                    OckamCommand::Transport(TransportCommand::Stop) => {
                        keep_going = false;
                        break;
                    }
                    _ => println!("unrecognized command"),
                }
            } // end match rx.try_recv()
        }
        keep_going
    }
}

pub struct UdpConnection {
    socket: UdpSocket,
}

impl UdpConnection {
    pub fn new(local: &str, remote: &str) -> Result<UdpConnection, String> {
        let socket = UdpSocket::bind(local).expect("couldn't bind to local socket");
        let remote_address = SocketAddrV4::from_str(remote).expect("bad remote address");
        let remote_address = SocketAddr::V4(remote_address);
        socket
            .connect(remote_address)
            .map_err(|_| "couldn't connect to remote address".to_string())?;
        socket
            .set_nonblocking(true)
            .map_err(|_| "couldn't set to non blocking".to_string())?;
        Ok(UdpConnection { socket })
    }

    pub fn address_as_string(&self) -> String {
        self.socket.local_addr().unwrap().to_string()
    }

    pub fn receive_message(
        &mut self,
        router_tx: &mut Sender<OckamCommand>,
        transport_tx: &mut Sender<OckamCommand>,
    ) -> bool {
        let mut buff = [0; 16348];
        match self.socket.recv(&mut buff) {
            Ok(_) => {
                match Message::decode(&buff) {
                    Ok((m, _unused)) => {
                        // send message to router
                        //todo if onward route is udp, send it now
                        if m.onward_route.addresses[0].a_type == AddressType::Udp {
                            transport_tx
                                .send(OckamCommand::Transport(TransportCommand::SendMessage(m)))
                                .is_ok()
                        } else {
                            router_tx
                                .send(OckamCommand::Router(RouterCommand::ReceiveMessage(m)))
                                .is_ok()
                        }
                    }
                    Err(_) => false,
                }
            }
            Err(e) => matches!(e.kind(), io::ErrorKind::WouldBlock),
        }
    }
}
