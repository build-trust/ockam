use serde::{Deserialize, Serialize};
use serde_bare;
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[repr(C)]
pub struct Message {
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
    Local,                  // type = 0
    SocketAddr(SocketAddr), // type = 1
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct Route {
    count: usize,
    addrs: Vec<Address>,
}

impl Route {
    pub fn from_u8_slice(u: &[u8]) -> Result<Self, String> {
        let mut addrs: Vec<Address> = vec![];
        let count = u[0] as usize;
        let mut offset = 1;
        for _i in 0..count {
            let addr_size = u[offset];
            offset += 1;
            match serde_bare::from_slice::<Address>(&u[offset..]) {
                Ok(a) => {
                    offset += addr_size as usize;
                    addrs.push(a);
                }
                Err(e) => {
                    return Err(format!("{:?}", e));
                }
            }
        }
        Ok(Route { count, addrs })
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, String> {
        let mut v: Vec<u8> = vec![];
        v.push(self.count as u8);
        for i in 0..self.count {
            match serde_bare::to_vec::<Address>(&self.addrs[i]) {
                Ok(mut va) => {
                    v.push(va.len() as u8);
                    v.append(&mut va);
                }
                Err(e) => {
                    return Err(format!("{:?}", e));
                }
            }
        }
        Ok(v)
    }
}

#[cfg(test)]
mod test {
    use crate::message::{Address, MessageBody, Route};
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[test]
    fn ip4_route_to_vec() {
        let ip1 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap());
        let ip2 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8081").unwrap());
        let route = Route {
            count: 2,
            addrs: vec![ip1, ip2],
        };
        let v = route.to_vec().unwrap();
        assert_eq!(
            v[0..],
            [2, 8, 1, 0, 127, 0, 0, 1, 144, 31, 8, 1, 0, 127, 0, 0, 1, 145, 31]
        );
    }

    #[test]
    fn ip4_route_from_slice() {
        let ip1 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8080").unwrap());
        let ip2 = Address::SocketAddr(SocketAddr::from_str("127.0.0.1:8081").unwrap());
        let route = Route {
            count: 2,
            addrs: vec![ip1, ip2],
        };
        let s = [
            2u8, 8, 1, 0, 127, 0, 0, 1, 144, 31, 8, 1, 0, 127, 0, 0, 1, 145, 31,
        ];
        match Route::from_u8_slice(&s) {
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
}
