#[allow(unused)]

pub mod transport {
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
        connections: std::collections::HashMap<String, UdpConnection>,
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
        ) -> UdpTransport {
            router_tx.send(OckamCommand::Router(RouterCommand::Register(
                AddressType::Udp,
                tx.clone(),
            )));
            UdpTransport {
                connections: std::collections::HashMap::new(),
                rx,
                tx,
                router_tx,
                buffer: [0; 16384],
            }
        }

        pub fn send_message(&mut self, mut m: Message) -> Result<(), String> {
            let mut udp = match self
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
            let mut local_addr: SocketAddr;
            match udp.socket.local_addr() {
                Ok(la) => {
                    local_addr = la;
                }
                Err(_unused) => return Err("send_message".to_string()),
            };

            if let Some(ra) =
                RouterAddress::from_address(Address::UdpAddress(udp.socket.local_addr().unwrap()))
            {
                m.return_route.addresses.insert(0, ra);
            }
            let mut v = vec![];
            Message::encode(m, &mut v);
            match udp.socket.send(v.as_slice()) {
                Ok(n) => Ok(()),
                Err(s) => Err("socket.send error".to_string()),
            }
        }

        pub fn poll(&mut self) -> bool {
            let mut got: bool = true;
            let mut keep_going = true;
            while got {
                got = false;
                for (_, mut c) in self.connections.iter_mut() {
                    keep_going = c.receive_message(&mut self.router_tx, &mut self.tx);
                }
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
                                Err(s) => return false,
                            };
                            println!("Added {} to transport", remote);
                            self.connections.insert(c.address_as_string(), c);
                        }
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
                        _ => {}
                    }
                } // end match rx.try_recv()
            }
            keep_going
        }
    }

    pub struct UdpConnection {
        socket: UdpSocket,
        remote_address: SocketAddr,
    }

    impl UdpConnection {
        pub fn new(local: &str, remote: &str) -> Result<UdpConnection, String> {
            let mut socket = UdpSocket::bind(local).expect("couldn't bind to local socket");
            let remote_address = SocketAddrV4::from_str(remote).expect("bad remote address");
            let remote_address = SocketAddr::V4(remote_address);
            match socket.connect(remote_address) {
                Ok(s) => {
                    socket.set_nonblocking(true);
                    Ok(UdpConnection {
                        socket,
                        remote_address,
                    })
                }
                Err(_a) => Err("couldn't connect to remote address".to_string()),
            }
        }

        pub fn address_as_string(&self) -> String {
            self.socket.local_addr().unwrap().to_string()
        }

        // pub fn send(&mut self, buff: &[u8]) -> Result<usize, String> {
        //     match self.socket.send(buff) {
        //         Ok(s) => Ok(s),
        //         Err(_unused) => Err("udp send failed".to_string()),
        //     }
        // }
        //
        // pub fn receive(&mut self, buff: &mut [u8]) -> Result<usize, String> {
        //     match self.socket.recv(buff) {
        //         Ok(s) => Ok(s),
        //         Err(e) => match e.kind() {
        //             io::ErrorKind::WouldBlock => Ok(0),
        //             _ => Err("udp receive error".to_string()),
        //         },
        //     }
        // }

        pub fn receive_message(
            &mut self,
            router_tx: &mut std::sync::mpsc::Sender<OckamCommand>,
            transport_tx: &mut std::sync::mpsc::Sender<OckamCommand>,
        ) -> bool {
            let mut buff = [0; 16348];
            match self.socket.recv(&mut buff) {
                Ok(s) => {
                    match Message::decode(&buff) {
                        Ok((mut m, _unused)) => {
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

    // impl MessageHandler for UdpConnection {
    //     fn message_handler(&mut self, mut m: Message) -> Result<(), String> {
    //         // pop first address
    //         // send message
    //         let router_address = m.onward_route.addresses.remove(0);
    //         match router_address.a_type {
    //             AddressType::Udp => match router_address.address {
    //                 Address::UdpAddress(ip, p) => {
    //                     let mut u: Vec<u8> = vec![];
    //                     match Message::encode(m, &mut u) {
    //                         Ok(_0) => (),
    //                         Err(_0) => return Err("failed to encode message".to_string()),
    //                     }
    //                     match self.socket.send(&u) {
    //                         Ok(_0) => Ok(()),
    //                         Err(_0) => Err("udp send failed".to_string()),
    //                     }
    //                 }
    //                 _ => {
    //                     assert!(false);
    //                 }
    //             },
    //             _ => Err("wrong address type".to_string()),
    //         }
    //     }
    //}
}

// #[cfg(test)]
// mod tests {
//     use crate::transport::*;
//     use ockam_message::message::*;
//     use ockam_router::router::{MessageHandler, Router};
//     use std::net::UdpSocket;
//     use std::sync::{Arc, Mutex};
//     use std::{thread, time};
//
//     fn recv_thread(addr: &str) {
//         let socket = UdpSocket::bind(addr).expect("couldn't bind to local socket");
//         let mut buff: [u8; 100] = [0; 100];
//         println!("calling recv");
//         match socket.recv(&mut buff) {
//             Ok(n) => println!(
//                 "received {} bytes: {}",
//                 n,
//                 std::str::from_utf8(&buff).expect("bad string")
//             ),
//             Err(_0) => println!("receive failed"),
//         }
//     }
//    #[test]
//     fn test_connect() {
//         let j: thread::JoinHandle<_> = thread::spawn(|| {
//             //println!("spawned");
//             recv_thread("127.0.0.1:4051")
//         });
//
//         let half_sec = time::Duration::from_millis(500);
//         thread::sleep(half_sec);
//
//         match UdpConnection::new("127.0.0.1:4050", "127.0.0.1:4051") {
//             Ok(mut t) => {
//                 println!("Connected");
//                 let buff = "hello ockam".as_bytes();
//                 match t.send(buff) {
//                     Ok(s) => println!("Sent {} bytes: {}", s, "hello ockam"),
//                     Err(_e) => println!("Send failed"),
//                 }
//             }
//             Err(s) => println!("Failed to connect {}", s),
//         }
//         j.join().expect("panic");
//     }
//
//     pub struct LocalProcess {
//         counter: u32,
//     }
//
//     impl MessageHandler for LocalProcess {
//         fn message_handler(&mut self, m: Message) -> Result<(), String> {
//             unimplemented!()
//         }
//     }
//
//     fn udp_responder(address: &str, router: Arc<Mutex<&mut Router>>) {
//         // register message handler
//         let mut udp_handler: Arc<Mutex<UdpConnection>>;
//         match UdpConnection::new_from_initiator(address) {
//             Ok(u) => {
//                 udp_handler = Arc::new(Mutex::new(u));
//                 let udp_connection = udp_handler.clone();
//                 match router
//                     .lock()
//                     .unwrap()
//                     .register_handler(udp_handler, AddressType::Udp)
//                 {
//                     Ok(()) => (),
//                     Err(s) => return,
//                 }
//
//                 loop {
//                     let mut u: [u8; 1024] = [0; 1024];
//                     let r = udp_connection.lock().unwrap().receive_message(&mut u);
//                     match r {
//                         Ok(message) => match router.lock().unwrap().route(message) {
//                             Ok(()) => {}
//                             Err(s) => {
//                                 println!("router.route failed");
//                                 return;
//                             }
//                         },
//                         Err(s) => {
//                             println!("receive_message failed {}", s);
//                         }
//                     }
//                 }
//             }
//             Err(s) => return,
//         }
//     }
//
//     fn test_handler() {}
// }
