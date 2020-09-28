#![allow(unused)]
pub mod router {
    use ockam_common::commands::ockam_commands::*;
    use ockam_message::message::*;
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};
    use std::{thread, time};

    pub struct Router {
        registry: Vec<Option<std::sync::mpsc::Sender<OckamCommand>>>,
        rx: std::sync::mpsc::Receiver<OckamCommand>,
    }

    pub enum MessageDirection {
        Send,
        Receive,
    }

    impl Router {
        pub fn new(rx: std::sync::mpsc::Receiver<OckamCommand>) -> Router {
            Router {
                registry: vec![Option::None; 256],
                rx,
            }
        }

        pub fn poll(&mut self) -> bool {
            let mut keep_going = true;
            let mut got = true;
            while got {
                got = false;
                match self.rx.try_recv() {
                    Ok(rc) => match rc {
                        OckamCommand::Router(RouterCommand::Stop) => {
                            println!("quit!");
                            got = true;
                            keep_going = false;
                            break;
                        }
                        OckamCommand::Router(RouterCommand::Register(a_type, tx)) => {
                            println!("Registering");
                            got = true;
                            self.registry[a_type as usize] = Option::Some(tx);
                        }
                        OckamCommand::Router(RouterCommand::ReceiveMessage(m)) => {
                            got = true;
                            self.route(m, MessageDirection::Receive);
                        }
                        OckamCommand::Router(RouterCommand::SendMessage(m)) => {
                            got = true;
                            self.route(m, MessageDirection::Send);
                        }
                        _ => println!("Router received bad command"),
                    },
                    Err(e) => {}
                }
            }
            keep_going
        }

        fn route(&mut self, m: Message, direction: MessageDirection) -> Result<(), String> {
            if m.onward_route.addresses.is_empty() {
                return Err("no route supplied".to_string());
            }

            let destination_address = m.onward_route.addresses[0].clone();
            let address_type = destination_address.a_type;
            let handler_tx = match &self.registry[address_type as usize] {
                Some(a) => a,
                None => return Err("no handler".to_string()),
            };
            match address_type {
                AddressType::Channel => match direction {
                    MessageDirection::Receive => {
                        handler_tx.send(OckamCommand::Channel(ChannelCommand::ReceiveMessage(m)));
                        Ok(())
                    }
                    MessageDirection::Send => {
                        handler_tx.send(OckamCommand::Channel(ChannelCommand::SendMessage(m)));
                        Ok(())
                    }
                },
                AddressType::Worker => match direction {
                    MessageDirection::Receive => {
                        handler_tx.send(OckamCommand::Worker(WorkerCommand::ReceiveMessage(m)));
                        Ok(())
                    }
                    MessageDirection::Send => {
                        handler_tx.send(OckamCommand::Worker(WorkerCommand::SendMessage(m)));
                        Ok(())
                    }
                },
                AddressType::Udp => {
                    handler_tx.send(OckamCommand::Transport(TransportCommand::SendMessage(m)));
                    Ok(())
                }
                _ => Err("not implemented".to_string()),
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::router::*;
//     use ockam_message::message::*;
//     use std::net::UdpSocket;
//     use std::net::{IpAddr, Ipv4Addr};
//     use std::str;
//     use std::sync::mpsc::channel;
//     use std::sync::{Arc, Mutex};
//     use std::{thread, time};

//     #[test]
//     fn test_udp_handler() {
//         let mut onward_addresses: Vec<Address> = vec![];
//         onward_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
//             0x8080,
//         ));
//         onward_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)),
//             0x7070,
//         ));
//         onward_addresses.push(Address::LocalAddress(
//             AddressType::Local as u8,
//             4,
//             LocalAddress {
//                 address: 0x00010203,
//             },
//         ));
//         let mut return_addresses: Vec<Address> = vec![];
//         return_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
//             0x8080,
//         ));
//         return_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)),
//             0x7070,
//         ));
//         return_addresses.push(Address::LocalAddress(
//             AddressType::Local as u8,
//             4,
//             LocalAddress {
//                 address: 0x00010203,
//             },
//         ));
//         let onward_route = Route {
//             addresses: onward_addresses,
//         };
//         let return_route = Route {
//             addresses: return_addresses,
//         };
//         let mut message_body = vec![0];
//         let mut msg = Box::new(Message {
//             onward_route,
//             return_route,
//             message_body,
//         });
//         let (tx, rx) = std::sync::mpsc::channel();
//         let join_router: thread::JoinHandle<_> = thread::spawn(|| Router::start(rx));
//
//         let udp_socket = UdpSocket::bind("127.0.0.1:4050").expect("couldn't bind to address");
//         let udp_handler: Arc<Mutex<dyn MessageHandler + Send>> =
//             Arc::new(Mutex::new(TestUdpHandler { socket: udp_socket }));
//         let cmd: commands::RouterCommand = commands::RouterCommand::Register(udp_handler, AddressType::Udp);
//         tx.send(cmd);
//         let cmd: RouterCommand = RouterCommand::Route(msg);
//         tx.send(cmd);
//         let cmd: RouterCommand = RouterCommand::None;
//         tx.send(cmd);
//         join_router.join();
//     }
// }

// #![allow(unused)]
// pub mod router {
//     use ockam_message::message::*;
//     use std::sync::mpsc::channel;
//     use std::sync::{Arc, Mutex};
//     use std::{thread, time};
//
//     pub struct RouterCommandRegister {
//         //registry: Option<Arc<Mutex<dyn MessageHandler + Send>>>,
//         registry: Option<Box<std::sync::mpsc::Sender<Message>>>,
//         address_type: AddressType,
//     }
//
//     pub enum RouterCommand {
//         None,
//         Register(std::sync::mpsc::Sender<Message>, AddressType),
//         //Register(std::sync::mpsc::Sender<RouterCommand>, AddressType),
//         Route(Message),
//         DeRegister(AddressType),
//     }
//
//     // pub trait MessageHandler {
//     //     fn message_handler(&mut self, m: Message) -> Result<(), String>;
//     // }
//
//     pub struct Router {
//         registry: Vec<Option<Arc<Mutex<std::sync::mpsc::Sender<Message>>>>>,
//     }
//
//     impl Router {
//         pub fn start(rx: std::sync::mpsc::Receiver<RouterCommand>) {
//             let mut router = Router {
//                 registry: vec![Option::None; 256],
//             };
//             let mut router_command = RouterCommand::None;
//             loop {
//                 match rx.try_recv() {
//                     Ok(rc) => {
//                         println!("got rx");
//                         match rc {
//                             RouterCommand::None => {
//                                 println!("quit!");
//                                 break;
//                             }
//                             RouterCommand::Register(r, a) => {
//                                 println!("Registering");
//                                 router.register_handler(r, a);
//                             }
//                             RouterCommand::Route(m) => {
//                                 println!("Routing");
//                                 router.route(m);
//                             }
//                             _ => {
//                                 println!("unknown command");
//                             }
//                         }
//                     }
//                     Err(e) => {
//                         thread::sleep(time::Duration::from_millis(100));
//                     }
//                 }
//             }
//         }
//
//         fn register_handler(
//             &mut self,
//             handler: Arc<Mutex<dyn MessageHandler + Send>>,
//             address_type: AddressType,
//         ) -> Result<(), String> {
//             self.registry[address_type as usize] = Option::Some(handler);
//             Ok(())
//         }
//
//         fn route(&mut self, m: Message) -> Result<(), String> {
//             // If there are no addresses, route to the controller
//             // Controller key is always 0
//             let handler_ref: Arc<Mutex<dyn MessageHandler + Send>>;
//             let mut address_type: u8 = 0;
//             let address: Address;
//             if !m.onward_route.addresses.is_empty() {
//                 address = m.onward_route.addresses[0];
//                 match address {
//                     Address::LocalAddress(t, _l, _unused) => {
//                         address_type = t as u8;
//                     }
//                     Address::UdpAddress(t, _l, _unused, _1) => {
//                         address_type = t as u8;
//                     }
//                     Address::TcpAddress(t, _l, _unused, _1) => {
//                         address_type = t as u8;
//                     }
//                 }
//             }
//             match &self.registry[address_type as usize] {
//                 Some(a) => {
//                     handler_ref = Arc::clone(a);
//                 }
//                 None => return Err("no handler".to_string()),
//             }
//             let r = handler_ref.lock().unwrap().message_handler(m);
//             match r {
//                 Ok(()) => Ok(()),
//                 Err(s) => Err(s),
//             }
//         }
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use crate::router::*;
//     use ockam_message::message::*;
//     use std::net::UdpSocket;
//     use std::net::{IpAddr, Ipv4Addr};
//     use std::str;
//     use std::sync::mpsc::channel;
//     use std::sync::{Arc, Mutex};
//     use std::{thread, time};
//
//     struct TestUdpHandler {
//         pub socket: UdpSocket,
//     }
//
//     impl MessageHandler for TestUdpHandler {
//         fn message_handler(&mut self, mut m: Message) -> Result<(), String> {
//             println!("In TestAddressHandler!");
//             Ok(())
//         }
//     }
//
//     #[test]
//     fn test_udp_handler() {
//         let mut onward_addresses: Vec<Address> = vec![];
//         onward_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
//             0x8080,
//         ));
//         onward_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)),
//             0x7070,
//         ));
//         onward_addresses.push(Address::LocalAddress(
//             AddressType::Local as u8,
//             4,
//             LocalAddress {
//                 address: 0x00010203,
//             },
//         ));
//         let mut return_addresses: Vec<Address> = vec![];
//         return_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
//             0x8080,
//         ));
//         return_addresses.push(Address::UdpAddress(
//             AddressType::Udp as u8,
//             7,
//             IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)),
//             0x7070,
//         ));
//         return_addresses.push(Address::LocalAddress(
//             AddressType::Local as u8,
//             4,
//             LocalAddress {
//                 address: 0x00010203,
//             },
//         ));
//         let onward_route = Route {
//             addresses: onward_addresses,
//         };
//         let return_route = Route {
//             addresses: return_addresses,
//         };
//         let mut message_body = vec![0];
//         let mut msg = Box::new(Message {
//             onward_route,
//             return_route,
//             message_body,
//         });
//         let (tx, rx) = std::sync::mpsc::channel();
//         let join_router: thread::JoinHandle<_> = thread::spawn(|| Router::start(rx));
//
//         let udp_socket = UdpSocket::bind("127.0.0.1:4050").expect("couldn't bind to address");
//         let udp_handler: Arc<Mutex<dyn MessageHandler + Send>> =
//             Arc::new(Mutex::new(TestUdpHandler { socket: udp_socket }));
//         let cmd: RouterCommand = RouterCommand::Register(udp_handler, AddressType::Udp);
//         tx.send(cmd);
//         let cmd: RouterCommand = RouterCommand::Route(msg);
//         tx.send(cmd);
//         let cmd: RouterCommand = RouterCommand::None;
//         tx.send(cmd);
//         join_router.join();
//     }
// }
