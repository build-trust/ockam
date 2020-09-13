// #![allow(unused)]
pub mod router {
    use ockam_message::message::*;
    use std::sync::{Arc, Mutex};

    pub trait MessageHandler {
        fn message_handler(&self, m: Box<Message>) -> Result<(), String>;
    }

    pub struct Router {
        registry: Vec<Option<Arc<Mutex<dyn MessageHandler + Send>>>>,
    }

    impl Router {
        pub fn new() -> Router {
            Router {
                registry: vec![Option::None; 256],
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
            // Pop the first address in the list
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
    use std::sync::{Arc, Mutex};

    struct TestUdpHandler {
        pub socket: UdpSocket,
    }

    impl MessageHandler for TestUdpHandler {
        fn message_handler(&self, _m: Box<Message>) -> Result<(), String> {
            println!("In TestAddressHandler!");
            Ok(())
        }
    }

    #[test]
    fn test_handler() {
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
