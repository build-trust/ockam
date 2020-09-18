#[allow(unused)]

pub mod transport {
    // use ockam_message::message::Address::UdpAddress;
    // use ockam_message::message::AddressType::Udp;
    use ockam_message::message::{Address, Codec, Message, Route};
    use ockam_router::router::MessageHandler;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::net::{SocketAddrV4, UdpSocket};
    use std::str::FromStr;
    use std::sync::Arc;

    pub struct Transport {}

    pub struct UdpConnection {
        socket: UdpSocket,
    }

    impl UdpConnection {
        pub fn new(local: &str, remote: &str) -> Result<UdpConnection, String> {
            let mut socket = UdpSocket::bind(local).expect("couldn't bind to local socket");
            let remote_socket = SocketAddrV4::from_str(remote).expect("bad remote address");
            match socket.connect(remote_socket) {
                Ok(s) => Ok(UdpConnection { socket }),
                Err(_a) => Err("couldn't connect to remote address".to_string()),
            }
        }

        pub fn send(&mut self, buff: &[u8]) -> Result<usize, String> {
            match self.socket.send(buff) {
                Ok(s) => Ok(s),
                Err(_0) => Err("udp send failed".to_string()),
            }
        }

        pub fn receive(&mut self, buff: &mut [u8]) -> Result<usize, String> {
            match self.socket.recv(buff) {
                Ok(s) => Ok(s),
                Err(_0) => Err("udp receive failed".to_string()),
            }
        }

        pub fn receive_message(&mut self, buff: &mut [u8]) -> Result<Box<Message>, String> {
            return match self.socket.recv(buff) {
                Ok(_0) => match Message::decode_boxed(buff) {
                    Ok(m) => Ok(m.0),
                    Err(s) => Err(s),
                },
                Err(_0) => Err("recv failed".to_string()),
            };
        }
    }

    impl MessageHandler for UdpConnection {
        fn message_handler(&self, mut m: Box<Message>) -> Result<(), String> {
            // pop first address
            // send message
            let address = m.onward_route.addresses.remove(0);
            match address {
                Address::UdpAddress(t, udp_address, port) => {
                    let mut u: Vec<u8> = vec![];
                    let mut msg = *m;
                    match Message::encode(&mut msg, &mut u) {
                        Ok(_0) => (),
                        Err(_0) => return Err("failed to encode message".to_string()),
                    }
                    match self.socket.send(&u) {
                        Ok(_0) => Ok(()),
                        Err(_0) => Err("udp send failed".to_string()),
                    }
                }
                _ => Err("wrong address type".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::*;
    use std::net::UdpSocket;
    use std::{thread, time};

    fn recv_thread(addr: &str) {
        let socket = UdpSocket::bind(addr).expect("couldn't bind to local socket");
        let mut buff: [u8; 100] = [0; 100];
        println!("calling recv");
        match socket.recv(&mut buff) {
            Ok(n) => println!(
                "received {} bytes: {}",
                n,
                std::str::from_utf8(&buff).expect("bad string")
            ),
            Err(_0) => println!("receive failed"),
        }
    }

    #[test]
    fn test_connect() {
        let j: thread::JoinHandle<_> = thread::spawn(|| {
            //println!("spawned");
            recv_thread("127.0.0.1:4051")
        });

        let half_sec = time::Duration::from_millis(500);
        thread::sleep(half_sec);

        match UdpConnection::new("127.0.0.1:4050", "127.0.0.1:4051") {
            Ok(mut t) => {
                println!("Connected");
                let buff = "hello ockam".as_bytes();
                match t.send(buff) {
                    Ok(s) => println!("Sent {} bytes: {}", s, "hello ockam"),
                    Err(_e) => println!("Send failed"),
                }
            }
            Err(s) => println!("Failed to connect {}", s),
        }
        j.join().expect("panic");
    }
}
