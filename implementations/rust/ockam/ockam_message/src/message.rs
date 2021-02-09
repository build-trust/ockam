use serde::{Deserialize, Serialize};
use serde_bare;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]

pub struct Message {
    version: u8,
    onward_route: Route,
    return_route: Route,
    message_body: MessageBody,
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
    addrs: Vec<Address>,
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
}
