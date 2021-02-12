use crate::error::TransportError;
use crate::transport_traits::Connection;
use ockam_core::Error;
use ockam_message::message::Message;
use serde_bare::Uint;

const MAX_MESSAGE_SIZE: usize = 2048;

pub struct Serializer {}

impl Serializer {
    pub async fn deserialize(mut c: Box<dyn Connection + Send>) -> Result<Message, TransportError> {
        let mut offset = 0;
        let mut message_length = 0;
        let mut message_buff = [0u8; MAX_MESSAGE_SIZE];

        loop {
            println!("*******************");
            let bytes_received = c.receive(&mut message_buff[offset..]).await?;

            // if message_length is 0, then decode the next byte(s) as message length
            if message_length == 0 {
                if let Ok(Uint(l)) = serde_bare::from_slice::<Uint>(&message_buff) {
                    if l as usize > MAX_MESSAGE_SIZE {
                        return Err(TransportError::IllFormedMessage);
                    }
                    message_length = l as usize;
                } else {
                    return Err(TransportError::IllFormedMessage);
                }
            }

            let remaining_msg_bytes = message_length - offset;
            offset += bytes_received;

            if bytes_received < remaining_msg_bytes {
                // not enough bytes to complete message
                continue;
            }

            // we have a complete message
            let mut l = 1;
            if message_length > 127 {
                l = 2;
            }
            return match serde_bare::from_slice::<Message>(&message_buff[l..(l + message_length)]) {
                Ok(m) => Ok(m),
                Err(_) => Err(TransportError::IllFormedMessage),
            };
        }
    }
    pub async fn serialize(mut c: Box<dyn Connection + Send>, m: Message) -> Result<(), Error> {
        return match serde_bare::to_vec::<Message>(&m) {
            Ok(mut vm) => {
                if vm.len() > MAX_MESSAGE_SIZE - 2 {
                    return Err(TransportError::IllFormedMessage.into());
                }
                let len = serde_bare::Uint(vm.len() as u64);
                let mut vl = serde_bare::to_vec::<serde_bare::Uint>(&len).unwrap();
                vl.append(&mut vm);
                return match c.send(&vl).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e.into()),
                };
            }
            Err(_) => Err(TransportError::IllFormedMessage.into()),
        };
    }
}

#[cfg(test)]
mod test {
    use crate::connection::TcpConnection;
    use crate::listener::TcpListener;
    use crate::Serializer;
    use ockam_message::message::{Address, Message, MessageBody, Route};
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::runtime::Builder;
    use tokio::task;
    use tokio::time::Duration;

    async fn ok_listener(a: String) {
        let r = TcpListener::new(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let connection = connection.unwrap();

        match Serializer::deserialize(connection).await {
            Ok(m) => {
                assert_eq!(
                    m,
                    Message {
                        version: 0,
                        onward_route: Route {
                            addrs: vec![
                                Address::SocketAddr(
                                    SocketAddr::from_str("127.0.0.1:8080").unwrap()
                                ),
                                Address::Local(b"0123".to_vec()),
                            ],
                        },
                        return_route: Route {
                            addrs: vec![
                                Address::SocketAddr(
                                    SocketAddr::from_str("127.0.0.1:8080").unwrap()
                                ),
                                Address::Local(b"0123".to_vec()),
                            ],
                        },
                        message_body: MessageBody::Ping,
                    }
                );
            }
            Err(e) => {
                panic!(format!("{:?}", e));
            }
        }
    }

    async fn ok_sender(a: String) {
        let mut connection = TcpConnection::new(std::net::SocketAddr::from_str(&a).unwrap());
        let r = connection.connect().await;
        assert!(!r.is_err());

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
        match Serializer::serialize(connection, m).await {
            Ok(()) => {}
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    async fn run_ok_test(address: String) {
        let a1 = address.clone();
        let j1 = task::spawn(async {
            let f = ok_listener(a1);
            f.await;
        });

        let a2 = address.clone();
        let j2 = task::spawn(async {
            let f = ok_sender(a2);
            f.await;
        });
        let (r1, r2) = tokio::join!(j1, j2);
        if r1.is_err() {
            panic!("{:?}", r1);
        }
        if r2.is_err() {
            panic!("{:?}", r2);
        }
    }

    async fn big_message_listener(a: String) {
        let r = TcpListener::new(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let connection = connection.unwrap();

        match Serializer::deserialize(connection).await {
            Ok(m) => {
                assert_eq!(
                    m,
                    Message {
                        version: 0,
                        onward_route: Route {
                            addrs: vec![
                                Address::SocketAddr(
                                    SocketAddr::from_str("127.0.0.1:8080").unwrap()
                                ),
                                Address::Local(b"0123".to_vec()),
                            ],
                        },
                        return_route: Route {
                            addrs: vec![
                                Address::SocketAddr(
                                    SocketAddr::from_str("127.0.0.1:8080").unwrap()
                                ),
                                Address::Local(b"0123".to_vec()),
                            ],
                        },
                        message_body: MessageBody::Payload(vec![0xfu8; 1024]),
                    }
                );
            }
            Err(e) => {
                panic!(format!("{:?}", e));
            }
        }
    }

    async fn big_message_sender(a: String) {
        let mut connection = TcpConnection::new(std::net::SocketAddr::from_str(&a).unwrap());
        let r = connection.connect().await;
        assert!(!r.is_err());

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
            message_body: MessageBody::Payload(vec![0xfu8; 1024]),
        };
        let mut vm = serde_bare::to_vec::<Message>(&m).unwrap();
        let len = serde_bare::Uint(vm.len() as u64);
        let mut vl = serde_bare::to_vec::<serde_bare::Uint>(&len).unwrap();
        vl.append(&mut vm);
        connection.send(&vl[0..512]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((1000.0) as u64)).await;
        connection.send(&vl[512..]).await.unwrap();
    }

    async fn run_big_message_test(address: String) {
        let a1 = address.clone();
        let j1 = task::spawn(async {
            let f = big_message_listener(a1);
            f.await;
        });

        let a2 = address.clone();
        let j2 = task::spawn(async {
            let f = big_message_sender(a2);
            f.await;
        });
        let (r1, r2) = tokio::join!(j1, j2);
        if r1.is_err() {
            panic!("{:?}", r1);
        }
        if r2.is_err() {
            panic!("{:?}", r2);
        }
    }

    #[test]
    fn ok_message() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            println!("run_ok_test starting...");
            run_ok_test(String::from("127.0.0.1:4050")).await;
            println!("run_ok_test done.");
            println!("run_big_message_test starting...");
            run_big_message_test(String::from("127.0.0.1:4050")).await;
            println!("run_big_message_test done.");
        });
    }
}
