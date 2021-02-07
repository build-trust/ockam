// Definition and implementation of an Ockam message and message components.
// Each message component, and the message overall, implements the "Codec" trait
// allowing it to be encoded/decoded for transmission over a transport.

use serde::{Deserialize, Serialize};
use std::convert::{Into, TryFrom, TryInto};
pub use std::io::{ErrorKind, Read, Write};
use std::net::SocketAddr;
use std::str::FromStr;
use std::string::String;
use url::Url;

/* Addresses */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[repr(C)]
pub enum Address {
    TcpAddress(SocketAddr),
    UdpAddress(SocketAddr),
    ChannelAddress(Vec<u8>),
    WorkerAddress(Vec<u8>),
}

pub fn hex_vec_from_str(s: &str) -> Result<Vec<u8>, String> {
    let mut hex: Vec<u8> = vec![];
    if s.len() % 2 != 0 {
        return Err("odd number of input chars".into());
    }
    for i in 0..s.len() {
        if 0 == i % 2 {
            let s2: &str = &s[i..(i + 2)];
            match u8::from_str_radix(s2, 16) {
                Ok(val) => {
                    hex.push(val);
                }
                _ => {
                    return Err("non-hex characters found in string".into());
                }
            }
        }
    }
    Ok(hex)
}

pub fn udp_address_from_str(s: &str) -> Result<Address, String> {
    match SocketAddr::from_str(s) {
        Ok(s) => Ok(Address::UdpAddress(s)),
        Err(_) => Err(format!("Invalid UDP address {}", s)),
    }
}

pub fn tcp_address_from_str(s: &str) -> Result<Address, String> {
    match SocketAddr::from_str(s) {
        Ok(s) => Ok(Address::TcpAddress(s)),
        Err(_) => Err(format!("Invalid TCP address {}", s)),
    }
}

pub fn worker_address_from_str(s: &str) -> Result<Address, String> {
    match hex_vec_from_str(s) {
        Ok(h) => Ok(Address::WorkerAddress(h)),
        Err(_unused) => Err(format!(
            "Worker address must be a hex number, with 2 chars per hex digit: {}",
            s
        )),
    }
}

pub fn channel_address_from_str(s: &str) -> Result<Address, String> {
    match hex_vec_from_str(s) {
        Ok(h) => Ok(Address::ChannelAddress(h)),
        Err(_unused) => Err(format!(
            "Channel address must be a hex number, with 2 chars per hex digit: {}",
            s
        )),
    }
}

impl TryFrom<String> for Address {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        match Url::parse(&s) {
            Ok(u) => {
                if !u.has_host() {
                    return Err(format!("invalid URI: {}", s));
                }

                match u.scheme() {
                    "udp" => {
                        let addr = u.as_str().trim().trim_start_matches("udp://");
                        udp_address_from_str(addr)
                    }
                    "tcp" => {
                        let addr = u.as_str().trim().trim_start_matches("tcp://");
                        tcp_address_from_str(addr)
                    }
                    "local" => {
                        let addr = u.as_str().trim().trim_start_matches("local://");
                        worker_address_from_str(addr)
                    }
                    "channel" => {
                        let addr = u.as_str().trim().trim_start_matches("channel://");
                        channel_address_from_str(addr)
                    }
                    _ => Err(format!("unsupported URL scheme for: {}", u.as_str())),
                }
            }
            Err(e) => Err(format!("failed to parse route part '{}': {:?}", s, e)),
        }
    }
}

impl TryInto<String> for Address {
    type Error = String;

    fn try_into(self) -> Result<String, Self::Error> {
        return match self {
            Address::UdpAddress(u) => Ok(format!("udp://{}", u.to_string())),
            Address::TcpAddress(u) => Ok(format!("tcp://{}", u.to_string())),
            Address::WorkerAddress(u) => Ok(format!("local://{}", hex::encode(u.as_slice()))),
            _ => Err("".into()),
        };
    }
}

pub enum HostAddressType {
    Ipv4 = 0,
    Ipv6 = 1,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_address_from_string() {
        let try_udp = Address::try_from("udp://127.0.0.1:8080".to_string());
        match try_udp {
            Ok(addr) => match addr {
                Address::UdpAddress(_) => {
                    let try_into: Result<String, String> = Address::try_into(addr);
                    match try_into {
                        Ok(s) => {
                            assert_eq!(s, "udp://127.0.0.1:8080".to_string());
                        }
                        _ => {}
                    }
                }
                _ => {
                    assert!(false);
                }
            },
            Err(_) => {
                assert!(false);
            }
        }
        let try_worker = Address::try_from("local://0a0b0c0d".to_string());
        match try_worker {
            Ok(addr) => match addr {
                Address::WorkerAddress(_) => {}
                _ => {
                    assert!(false);
                }
            },
            Err(_) => {
                assert!(false);
            }
        }
        let try_worker = Address::try_from("local://0a0b0c0".to_string());
        match try_worker {
            Ok(_) => {
                assert!(false);
            }
            Err(_) => {}
        }
        let try_worker = Address::try_from("local://0a0b0c0g".to_string());
        match try_worker {
            Ok(_) => {
                assert!(false);
            }
            Err(_) => {}
        }
        let try_worker = Address::try_from("local://0a0b0c0d050607".to_string());
        match try_worker {
            Ok(_) => {}
            Err(_) => {
                assert!(false);
            }
        }
    }
}
