use serde::{Deserialize, Serialize};
use serde_bare::Uint;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]

pub struct RouterMessage {
    pub version: u8,
    pub onward_route: Route,
    pub return_route: Route,
    pub payload: Vec<u8>,
}

pub const ROUTER_MSG_PING: u8 = 0;
pub const ROUTER_MSG_PONG: u8 = 1;
pub const ROUTER_MSG_PAYLOAD: u8 = 2;

pub const ROUTER_ADDRESS_LOCAL: Uint = serde_bare::Uint(0);
pub const ROUTER_ADDRESS_TCP: Uint = serde_bare::Uint(1);

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct RouterAddress {
    pub address_type: Uint,
    pub address: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Route {
    pub addrs: Vec<RouterAddress>,
}

#[cfg(test)]

mod test {
    use crate::message::{
        Route, RouterAddress, RouterMessage, ROUTER_ADDRESS_LOCAL, ROUTER_ADDRESS_TCP,
    };
    use serde_bare::Uint;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn address() {
        let sa = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sa_as_vec = serde_bare::to_vec::<SocketAddr>(&sa).unwrap();
        let ra = RouterAddress {
            address_type: ROUTER_ADDRESS_TCP,
            address: sa_as_vec,
        };
        let ra_serialized = serde_bare::to_vec::<RouterAddress>(&ra).unwrap();
        let ra_deserialized = serde_bare::from_slice::<RouterAddress>(&ra_serialized).unwrap();
        assert_eq!(ra_deserialized, ra);
    }

    #[test]
    fn ip4_route_to_vec() {
        let sa1 = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sa1_as_vec = serde_bare::to_vec::<SocketAddr>(&sa1).unwrap();
        let ra1 = RouterAddress {
            address_type: ROUTER_ADDRESS_TCP,
            address: sa1_as_vec,
        };
        let sa2 = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sa2_as_vec = serde_bare::to_vec::<SocketAddr>(&sa2).unwrap();
        let ra2 = RouterAddress {
            address_type: ROUTER_ADDRESS_TCP,
            address: sa2_as_vec,
        };

        let route = Route {
            addrs: vec![ra1, ra2],
        };
        let v = serde_bare::to_vec::<Route>(&route).unwrap();
        assert_eq!(
            v[0..],
            [2, 1, 7, 0, 127, 0, 0, 1, 144, 31, 1, 7, 0, 127, 0, 0, 1, 144, 31]
        );
    }

    #[test]
    fn ip4_route_from_slice() {
        let sa1 = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sa1_as_vec = serde_bare::to_vec::<SocketAddr>(&sa1).unwrap();
        let ra1 = RouterAddress {
            address_type: ROUTER_ADDRESS_TCP,
            address: sa1_as_vec,
        };
        let sa2 = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sa2_as_vec = serde_bare::to_vec::<SocketAddr>(&sa2).unwrap();
        let ra2 = RouterAddress {
            address_type: ROUTER_ADDRESS_TCP,
            address: sa2_as_vec,
        };
        let route = Route {
            addrs: vec![ra1, ra2],
        };

        let s = [
            2u8, 1, 7, 0, 127, 0, 0, 1, 144, 31, 1, 7, 0, 127, 0, 0, 1, 144, 31,
        ];
        match serde_bare::from_slice::<Route>(&s) {
            Ok(r) => {
                assert_eq!(r, route);
            }
            _ => {
                panic!("Message crate: test ip4_route_from_slice failed");
            }
        }
    }

    #[test]
    fn test_message() {
        let sock_addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
        let sock_addr_vec = serde_bare::to_vec::<SocketAddr>(&sock_addr).unwrap();

        let local_addr = b"printer".to_vec();

        let m = RouterMessage {
            version: 1,
            onward_route: Route {
                addrs: vec![
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_TCP,
                        address: sock_addr_vec.clone(),
                    },
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_LOCAL,
                        address: local_addr.clone(),
                    },
                ],
            },
            return_route: Route {
                addrs: vec![
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_TCP,
                        address: sock_addr_vec,
                    },
                    RouterAddress {
                        address_type: ROUTER_ADDRESS_LOCAL,
                        address: local_addr,
                    },
                ],
            },
            payload: b"hello".to_vec(),
        };

        let v = serde_bare::to_vec::<RouterMessage>(&m).unwrap();
        let m2 = serde_bare::from_slice::<RouterMessage>(&v).unwrap();
        assert_eq!(m, m2)
    }

    #[test]
    fn varint() {
        let i1 = serde_bare::Uint(255 as u64);
        let i2 = serde_bare::Uint(127 as u64);
        let i3 = serde_bare::Uint(128 as u64);

        let mut v1 = serde_bare::to_vec::<serde_bare::Uint>(&i1).unwrap();
        assert_eq!(v1.len(), 2);
        v1.append(&mut vec![1u8, 2, 3, 4]);

        let mut v2 = serde_bare::to_vec::<serde_bare::Uint>(&i2).unwrap();
        assert_eq!(v2.len(), 1);
        v2.append(&mut vec![1u8, 2, 3, 4]);

        let mut v3 = serde_bare::to_vec::<serde_bare::Uint>(&i3).unwrap();
        assert_eq!(v3.len(), 2);
        v3.append(&mut vec![1u8, 2, 3, 4]);

        let Uint(i1) = serde_bare::from_slice::<serde_bare::Uint>(&v1).unwrap();
        let Uint(i2) = serde_bare::from_slice::<serde_bare::Uint>(&v2).unwrap();
        let Uint(i3) = serde_bare::from_slice::<serde_bare::Uint>(&v3).unwrap();

        assert_eq!(i1, 255);
        assert_eq!(i2, 127);
        assert_eq!(i3, 128);
    }
}
