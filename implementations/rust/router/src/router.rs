#![allow(unused)]
pub mod router {
    use ockam_message::message::*;
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};
    use std::{thread, time};

    pub struct RouterCommandRegister {
        registry: Option<Arc<Mutex<dyn MessageHandler + Send>>>,
        address_type: AddressType,
    }

    pub enum RouterCommand {
        None,
        Register(Arc<Mutex<dyn MessageHandler + Send>>, AddressType),
        Route(Box<Message>),
        DeRegister(AddressType),
    }

    pub trait MessageHandler {
        fn message_handler(&mut self, m: Box<Message>) -> Result<(), String>;
    }

    pub struct Router {
        registry: Vec<Option<Arc<Mutex<dyn MessageHandler + Send>>>>,
    }

    impl Router {
        pub fn start(rx: std::sync::mpsc::Receiver<RouterCommand>) {
            let mut router = Router {
                registry: vec![Option::None; 256],
            };
            let mut router_command = RouterCommand::None;
            loop {
                match rx.try_recv() {
                    Ok(rc) => {
                        println!("got rx");
                        match rc {
                            RouterCommand::None => {
                                println!("quit!");
                                break;
                            }
                            RouterCommand::Register(r, a) => {
                                println!("Registering");
                                router.register_handler(r, a);
                            }
                            RouterCommand::Route(m) => {
                                println!("Routing");
                                router.route(m);
                            }
                            _ => {
                                println!("got command");
                            }
                        }
                    }
                    Err(e) => {
                        thread::sleep(time::Duration::from_millis(100));
                    }
                }
            }
        }

        pub fn register_handler(
            &mut self,
            handler: Arc<Mutex<dyn MessageHandler + Send>>,
            address_type: AddressType,
        ) -> Result<(), String> {
            self.registry[address_type as usize] = Option::Some(handler);
            Ok(())
        }

        pub fn route(&mut self, m: Box<Message>) -> Result<(), String> {
            // If there are no addresses, route to the controller
            // Controller key is always 0
            let handler_ref: Arc<Mutex<dyn MessageHandler + Send>>;
            let mut address_type: u8 = 0;
            let address: Address;
            if !m.onward_route.addresses.is_empty() {
                address = m.onward_route.addresses[0];
                match address {
                    Address::LocalAddress(t, _0) => {
                        address_type = t as u8;
                    }
                    Address::UdpAddress(t, _0, _1) => {
                        address_type = t as u8;
                    }
                    Address::TcpAddress(t, _0, _1) => {
                        address_type = t as u8;
                    }
                }
            }
            match &self.registry[address_type as usize] {
                Some(a) => {
                    handler_ref = Arc::clone(a);
                }
                None => return Err("no handler".to_string()),
            }
            let r = handler_ref.lock().unwrap().message_handler(m);
            match r {
                Ok(()) => Ok(()),
                Err(s) => Err(s),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::router::*;
    use ockam_message::message::*;
    use std::net::UdpSocket;
    use std::net::{IpAddr, Ipv4Addr};
    use std::str;
    use std::sync::mpsc::channel;
    use std::sync::{Arc, Mutex};
    use std::{thread, time};

    struct TestUdpHandler {
        pub socket: UdpSocket,
    }

    impl MessageHandler for TestUdpHandler {
        fn message_handler(&mut self, mut m: Box<Message>) -> Result<(), String> {
            println!("In TestAddressHandler!");
            Ok(())
        }
    }

    struct TestLocalHandler {
        payload: String,
    }

    impl MessageHandler for TestLocalHandler {
        fn message_handler(&mut self, mut m: Box<Message>) -> Result<(), String> {
            let s = match str::from_utf8(&m.message_body[..]) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            self.payload = s.to_string();
            println!("payload is {}", self.payload);
            Ok(())
        }
    }

    #[test]
    fn test_udp_handler() {
        let mut onward_addresses: Vec<Address> = vec![];
        onward_addresses.push(Address::UdpAddress(
            AddressType::Udp,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            0x8080,
        ));
        onward_addresses.push(Address::UdpAddress(
            AddressType::Udp,
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)),
            0x7070,
        ));
        onward_addresses.push(Address::LocalAddress(
            AddressType::Local,
            LocalAddress {
                address: 0x00010203,
            },
        ));
        let mut return_addresses: Vec<Address> = vec![];
        return_addresses.push(Address::UdpAddress(
            AddressType::Udp,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            0x8080,
        ));
        return_addresses.push(Address::UdpAddress(
            AddressType::Udp,
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)),
            0x7070,
        ));
        return_addresses.push(Address::LocalAddress(
            AddressType::Local,
            LocalAddress {
                address: 0x00010203,
            },
        ));
        let onward_route = Route {
            addresses: onward_addresses,
        };
        let return_route = Route {
            addresses: return_addresses,
        };
        let message_body = vec![0];
        let msg = Box::new(Message {
            onward_route,
            return_route,
            message_body,
        });
        let (tx, rx) = std::sync::mpsc::channel();
        let join_router: thread::JoinHandle<_> = thread::spawn(|| Router::start(rx));

        let udp_socket = UdpSocket::bind("127.0.0.1:4050").expect("couldn't bind to address");
        let udp_handler: Arc<Mutex<dyn MessageHandler + Send>> =
            Arc::new(Mutex::new(TestUdpHandler { socket: udp_socket }));
        let cmd: RouterCommand = RouterCommand::Register(udp_handler, AddressType::Udp);
        tx.send(cmd);
        let cmd: RouterCommand = RouterCommand::Route(msg);
        tx.send(cmd);
        let cmd: RouterCommand = RouterCommand::None;
        tx.send(cmd);
        join_router.join();
    }
}
