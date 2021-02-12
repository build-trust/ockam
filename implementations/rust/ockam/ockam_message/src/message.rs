use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]

pub struct Message {
    pub version: u8,
    pub onward_route: Route,
    pub return_route: Route,
    pub message_body: MessageBody,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]
pub enum MessageBody {
    Ping,
    Pong,
    Payload(Vec<u8>),
    RequestChannel,
    KeyAgreementM2,
    KeyAgreementM3,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]
pub enum Address {
    Local(Vec<u8>),         // type = 0
    SocketAddr(SocketAddr), // type = 1
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Route {
    pub addrs: Vec<Address>,
}

#[cfg(test)]

mod test {
    use crate::message::{Address, Message, MessageBody, Route};
    use serde::{Deserialize, Serialize};
    use serde_bare::Uint;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn ip4_route_to_vec() {
        let ip1 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap());
        let ip2 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8081").unwrap());
        let route = Route {
            addrs: vec![ip1, ip2],
        };
        let v = serde_bare::to_vec::<Route>(&route).unwrap();
        println!("{:?}", v);
        assert_eq!(
            v[0..],
            [2, 1, 0, 127, 0, 0, 1, 144, 31, 1, 0, 127, 0, 0, 1, 145, 31]
        );
    }

    #[test]
    fn ip4_route_from_slice() {
        let ip1 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap());
        let ip2 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8081").unwrap());
        let route = Route {
            addrs: vec![ip1, ip2],
        };
        let s = [
            2u8, 1, 0, 127, 0, 0, 1, 144, 31, 1, 0, 127, 0, 0, 1, 145, 31,
        ];
        match serde_bare::from_slice::<Route>(&s) {
            Ok(r) => {
                assert_eq!(r, route);
            }
            _ => {
                assert!(false);
            }
        }
    }

    #[test]
    fn test_payload() {
        let message_body = MessageBody::Payload(b"hello".to_vec());
        match serde_bare::to_vec(&message_body) {
            Ok(mbv) => {
                assert_eq!(mbv, vec![2, 5, 104, 101, 108, 108, 111]);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn routes() {
        let r = Route {
            addrs: vec![
                Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap()),
                Address::Local(b"0123".to_vec()),
            ],
        };
        #[derive(Serialize, Deserialize)]
        struct Rs {
            r1: Route,
            r2: Route,
        };
        let s = Rs {
            r1: r.clone(),
            r2: r,
        };
        let v = serde_bare::to_vec::<Rs>(&s).unwrap();
        println!("{:?}", v);
    }

    #[test]
    fn test_message() {
        let m = Message {
            version: 0,
            onward_route: Route {
                addrs: vec![
                    Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap()),
                    Address::Local(b"0123".to_vec()),
                ],
            },
            return_route: Route {
                addrs: vec![
                    Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap()),
                    Address::Local(b"0123".to_vec()),
                ],
            },
            message_body: MessageBody::Ping,
        };

        let mut v = serde_bare::to_vec::<Message>(&m).unwrap();
        println!("{} {:?}", v.len(), v);

        let l = v.len();
        let ul = serde_bare::Uint(l as u64);
        let mut vl = serde_bare::to_vec::<serde_bare::Uint>(&ul).unwrap();
        vl.append(&mut v);
        assert_eq!(
            vl,
            [
                32, 0, 2, 1, 0, 127, 0, 0, 1, 144, 31, 0, 4, 48, 49, 50, 51, 2, 1, 0, 127, 0, 0, 1,
                144, 31, 0, 4, 48, 49, 50, 51, 0
            ]
        );
        let m2 = serde_bare::from_slice::<Message>(&[
            0, 2, 1, 0, 127, 0, 0, 1, 144, 31, 0, 4, 48, 49, 50, 51, 2, 1, 0, 127, 0, 0, 1, 144,
            31, 0, 4, 48, 49, 50, 51, 0,
        ])
        .unwrap();
        assert_eq!(m, m2);
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
