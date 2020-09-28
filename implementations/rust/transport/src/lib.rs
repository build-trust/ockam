#[allow(unused)]

pub mod transport {
    use ockam_common::commands::ockam_commands::RouterCommand::ReceiveMessage;
    use ockam_common::commands::ockam_commands::*;
    use ockam_common::commands::*;
    use ockam_message::message::*;
    use ockam_router::router::Router;
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::net::{SocketAddrV4, UdpSocket};
    use std::str;
    use std::str::FromStr;
    use std::sync::mpsc::channel;
    use std::sync::Arc;
    use std::{io, thread, time};

    pub struct UdpTransport {
        socket: UdpSocket,
        rx: std::sync::mpsc::Receiver<OckamCommand>,
        tx: std::sync::mpsc::Sender<OckamCommand>,
        router_tx: std::sync::mpsc::Sender<OckamCommand>,
        buffer: [u8; 16384],
    }

    impl UdpTransport {
        pub fn new(
            rx: std::sync::mpsc::Receiver<OckamCommand>,
            tx: std::sync::mpsc::Sender<OckamCommand>,
            router_tx: std::sync::mpsc::Sender<OckamCommand>,
            local_address: &str,
        ) -> Result<UdpTransport, String> {
            // Try to create socket at given address
            match UdpSocket::bind(local_address) {
                Ok(socket) => {
                    socket.set_nonblocking(true);
                    // Register address type with Router
                    router_tx.send(OckamCommand::Router(RouterCommand::Register(
                        AddressType::Udp,
                        tx.clone(),
                    )));
                    println!("created udp transport bound to {}", local_address);
                    Ok(UdpTransport {
                        socket,
                        rx,
                        tx,
                        router_tx,
                        buffer: [0; 16384],
                    })
                }
                Err(_unused) => {
                    println!("failed to create socket");
                    Err("failed to create socket".to_string())
                }
            }
        }

        pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
            println!("Onward route:");
            m.onward_route.print_route();
            let remote_address = m.onward_route.addresses.remove(0);

            match self.socket.local_addr() {
                Ok(la) => match RouterAddress::from_address(Address::UdpAddress(la)) {
                    Some(ra) => {
                        m.return_route.addresses.insert(0, ra);
                        let mut v = vec![];
                        Message::encode(m, &mut v);
                        // println!("sending:");
                        // let b: Vec<u8> = v[0..].to_vec();
                        // println!("{:?}", b);
                        match self
                            .socket
                            .send_to(v.as_slice(), remote_address.address.as_string())
                        {
                            Ok(n) => Ok(()),
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
            let mut buff = [0; 16348];
            match self.socket.recv_from(&mut buff) {
                Ok((s, a)) => {
                    // println!("received:");
                    // let b: Vec<u8> = buff[0..100].to_vec();
                    // println!("{:?}", b);
                    match Message::decode(&buff[0..s]) {
                        Ok((mut m, _unused)) => {
                            println!("onward route:");
                            m.onward_route.print_route();
                            if m.onward_route.addresses[0].a_type == AddressType::Udp {
                                match self.send_message(m) {
                                    Err(s) => Err(s),
                                    Ok(()) => Ok(true),
                                }
                            } else {
                                match self.router_tx.send(OckamCommand::Router(ReceiveMessage(m))) {
                                    Ok(_unused) => Ok(true),
                                    Err(s) => Err("send to router failed".to_string()),
                                }
                            }
                        }
                        _ => Err("decode failed".to_string()),
                    }
                }
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
                    Err(e) => {
                        keep_going = false;
                    }
                }
            }

            got = true;
            while got && keep_going {
                got = false;
                if let Ok(tc) = self.rx.try_recv() {
                    match tc {
                        OckamCommand::Transport((TransportCommand::SendMessage(mut m))) => {
                            self.send_message(m);
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
}
