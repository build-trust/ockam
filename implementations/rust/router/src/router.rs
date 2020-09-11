#![allow(unused)]
pub mod router {
    use ockam_message::message::*;
    use std::collections::HashMap;
    use std::io::{Error, ErrorKind, Read, Write};
    use std::ops::Add;
    use std::sync::{Arc, Mutex};
    use std::thread;

    pub trait MessageHandler {
        fn message_handler(&self, m: Box<Message>, address: Address) -> Result<(), std::io::Error>;
    }

    pub struct Router {
        registry: HashMap<u64, Arc<Mutex<dyn MessageHandler + Send>>>,
    }

    impl Router {
        pub fn new() -> Router {
            Router {
                registry: HashMap::new(),
            }
        }

        pub fn register_handler(
            &mut self,
            handler: Arc<Mutex<dyn MessageHandler + Send>>,
            address_type: AddressType,
        ) -> Result<(), String> {
            let mut k: u64 = address_type as u64;
            self.registry.insert(k, handler);
            Ok(())
        }

        pub fn route(&mut self, mut m: Box<Message>) -> Result<(), String> {
            // Pop the first address in the list
            // If there are no addresses, route to the controller
            // Controller key is always 0
            let mut key: u64 = 0;
            let address: Address;
            if !m.onward_route.addresses.is_empty() {
                address = m.onward_route.addresses.remove(0);
                match address {
                    Address::LocalAddress(l) => {
                        key = AddressType::Local as u64;
                    }
                    Address::UdpAddress(ip, port) => {
                        key = AddressType::Udp as u64;
                    }
                    Address::TcpAddress(ip, port) => {
                        key = AddressType::Tcp as u64;
                    }
                }
            } else {
                address = Address::LocalAddress(LocalAddress {
                    length: 0,
                    address: 0,
                });
            }
            if !self.registry.contains_key(&key) {
                return Err("Not Implemented".to_string());
            }

            match self.registry.get_mut(&key) {
                Some(r) => {
                    let r = Arc::clone(r);
                    let j: thread::JoinHandle<_> = thread::spawn(move || {
                        r.lock().unwrap().message_handler(m, address);
                    });

                    j.join().expect("panic");
                    Ok(())
                }
                None => Err("key not found".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::router::*;
    use ockam_message::message::*;
    use std::collections::HashMap;
    use std::io::{Error, ErrorKind, Read, Write};
    use std::net::UdpSocket;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::ops::Add;
    use std::sync::{Arc, Mutex};

    struct TestUdpHandler {
        pub socket: UdpSocket,
    }

    impl MessageHandler for TestUdpHandler {
        fn message_handler(
            &self,
            mut m: Box<Message>,
            address: Address,
        ) -> Result<(), std::io::Error> {
            println!("In TestAddressHandler!");
            Ok(())
        }
    }

    #[test]
    fn test_handler() {
        let mut onward_addresses: Vec<Address> = vec![];
        onward_addresses.push(Address::UdpAddress(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            0x8080,
        ));
        onward_addresses.push(Address::UdpAddress(
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 10)),
            0x7070,
        ));
        onward_addresses.push(Address::LocalAddress(LocalAddress {
            length: 4,
            address: 0x00010203,
        }));
        let mut return_addresses: Vec<Address> = vec![];
        return_addresses.push(Address::UdpAddress(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            0x8080,
        ));
        return_addresses.push(Address::UdpAddress(
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 11)),
            0x7070,
        ));
        return_addresses.push(Address::LocalAddress(LocalAddress {
            length: 4,
            address: 0x00010203,
        }));
        let onward_route = Route {
            addresses: onward_addresses,
        };
        let return_route = Route {
            addresses: return_addresses,
        };
        let mut message_body = vec![0];
        let mut msg = Box::new(Message {
            onward_route,
            return_route,
            message_body,
        });
        let mut router: Router = Router::new();
        let udp_socket = UdpSocket::bind("127.0.0.1:4050").expect("couldn't bind to address");
        let udp_handler: Arc<Mutex<dyn MessageHandler + Send>> =
            Arc::new(Mutex::new(TestUdpHandler { socket: udp_socket }));
        match router.register_handler(udp_handler, AddressType::Udp) {
            Ok(()) => {
                println!("udp handler registered");
            }
            Err(s) => {
                println!("{}", s);
                return;
            }
        }
        match router.route(msg) {
            Ok(()) => println!("success!"),
            Err(s) => println!("{}", s),
        }
    }
}
