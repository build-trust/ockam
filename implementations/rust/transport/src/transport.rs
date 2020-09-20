#[allow(unused)]

pub mod transport {
    use ockam_common::commands::commands::*;
    use ockam_common::commands::*;
    use ockam_message::message::*;
    use ockam_router::router::Router;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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

        pub fn poll(&mut self) -> bool {
            let mut got: bool = true;
            let mut keep_going = true;
            let mut buff = [0; 16348];
            while got {
                got = false;
                for (_, mut c) in self.connections.iter_mut() {
                    match c.receive(&mut buff) {
                        Ok(size) => {
                            if size > 0 {
                                match Message::decode(&buff) {
                                    Ok((m, _0)) => {
                                        // // send message to router
                                        // let cmd = RouterCommand::Route(m);
                                        // router_tx.send(cmd);
                                        // got = true;
                                        println!(
                                            "received {}",
                                            str::from_utf8(&m.message_body).unwrap()
                                        );
                                    }
                                    Err(s) => {
                                        println!("message decode failed");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("udp receive error");
                            break;
                        }
                    }
                }
            }

            got = true;
            while got {
                got = false;
                match self.rx.try_recv() {
                    Ok(tc) => match tc {
                        OckamCommand::Transport(TransportCommand::Add(local, remote)) => {
                            got = true;
                            println!("creating udp connection");
                            let c = match UdpConnection::new(&local, &remote) {
                                Ok(c) => c,
                                Err(s) => return false,
                            };
                            println!("Added {} to transport", c.address_as_string());
                            self.connections.insert(c.address_as_string(), c);
                        }
                        OckamCommand::Transport((TransportCommand::Send(m))) => {
                            let mut udp = match self
                                .connections
                                .get_mut(&m.onward_route.addresses[0].address.to_string())
                            {
                                Some(c) => c,
                                None => {
                                    println!("connection not found");
                                    return false;
                                }
                            };
                            let mut v = vec![];
                            Message::encode(m, &mut v);
                            match udp.socket.send(v.as_slice()) {
                                Ok(n) => {
                                    println!("Sent {} bytes!", n);
                                }
                                Err(s) => {
                                    println!("socket.send error {}", s);
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
                    },
                    _ => {}
                } // end match rx.try_recv()
            }
            return keep_going;
        }
    }

    pub struct UdpConnection {
        socket: UdpSocket,
    }

    impl UdpConnection {
        pub fn new(local: &str, remote: &str) -> Result<UdpConnection, String> {
            let mut socket = UdpSocket::bind(local).expect("couldn't bind to local socket");
            let remote_address = SocketAddrV4::from_str(remote).expect("bad remote address");
            match socket.connect(remote_address) {
                Ok(s) => {
                    socket.set_nonblocking(true);
                    Ok(UdpConnection { socket })
                }
                Err(_a) => Err("couldn't connect to remote address".to_string()),
            }
        }

        pub fn address_as_string(&self) -> String {
            let mut s = self.socket.local_addr().unwrap().to_string();
            return s;
        }
        // pub fn new_from_initiator(local: &str) -> Result<UdpConnection, String> {
        //     let mut socket = UdpSocket::bind(local).expect("couldn't bind to local socket");
        //     let mut u: [u8; 1024] = [0; 1024];
        //     match socket.recv_from(&mut u) {
        //         Ok((_0, remote_address)) => match socket.connect(remote_address) {
        //             Ok(s) => Ok(UdpConnection { socket }),
        //             Err(_0) => Err("connect failed".to_string()),
        //         },
        //         Err(_0) => Err("recv_from failed".to_string()),
        //     }
        // }

        pub fn send(&mut self, buff: &[u8]) -> Result<usize, String> {
            match self.socket.send(buff) {
                Ok(s) => Ok(s),
                Err(_0) => Err("udp send failed".to_string()),
            }
        }

        pub fn receive(&mut self, buff: &mut [u8]) -> Result<usize, String> {
            match self.socket.recv(buff) {
                Ok(s) => Ok(s),
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => Ok(0),
                    _ => Err("udp receive error".to_string()),
                },
            }
        }

        pub fn receive_message(&mut self, buff: &mut [u8]) -> Result<Message, String> {
            return match self.socket.recv(buff) {
                Ok(_0) => match Message::decode(buff) {
                    Ok(m) => Ok(m.0),
                    Err(s) => Err(s),
                },
                Err(_0) => Err("recv failed".to_string()),
            };
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
