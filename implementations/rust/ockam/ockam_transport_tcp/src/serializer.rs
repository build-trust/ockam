use crate::error::TransportError;
use crate::transport_traits::Connection;
use ockam_core::Error;
use ockam_message::message::Message;
use serde_bare::Uint;

const MAX_MESSAGE_SIZE: usize = 2048;

pub struct Serializer {
    message_buff: Vec<u8>,
    message_length: usize,
    connection: Box<dyn Connection + Send>,
}

impl Serializer {
    pub fn new(connection: Box<dyn Connection + Send>) -> Self {
        Serializer {
            message_buff: vec![],
            message_length: 0,
            connection,
        }
    }

    pub async fn deserialize(&mut self) -> Result<Message, TransportError> {
        loop {
            let mut recv_buff = [0u8; MAX_MESSAGE_SIZE];

            // first see if we have a complete message from the last call
            // if not, read additional bytes
            if self.message_buff.len() <= self.message_length {
                let bytes_received = self.connection.receive(&mut recv_buff).await?;
                if 0 == bytes_received {
                    println!("00000000000000000");
                }
                self.message_buff
                    .append(&mut recv_buff[0..bytes_received].to_vec());
            }

            if self.message_length == 0 {
                let Uint(len) = serde_bare::from_slice::<Uint>(&recv_buff[0..]).unwrap();
                self.message_length = len as usize;
                self.message_buff.remove(0);
                if len > 127 {
                    self.message_buff.remove(0);
                }
            }

            // see if we have a complete message
            if self.message_length <= self.message_buff.len() {
                // we have a complete message
                match serde_bare::from_slice::<Message>(&self.message_buff) {
                    Ok(m) => {
                        // scoot any remaining bytes to the beginning of the buffer
                        for i in 0..self.message_buff.len() - self.message_length {
                            self.message_buff[i] = self.message_buff[i + self.message_length];
                        }
                        self.message_buff
                            .truncate(self.message_buff.len() - self.message_length);
                        self.message_length = 0;
                        return Ok(m);
                    }
                    Err(_) => {
                        return Err(TransportError::IllFormedMessage);
                    }
                }
            }
        }
    }
    pub async fn serialize(&mut self, m: Message) -> Result<(), Error> {
        return match serde_bare::to_vec::<Message>(&m) {
            Ok(mut vm) => {
                if vm.len() > MAX_MESSAGE_SIZE - 2 {
                    return Err(TransportError::IllFormedMessage.into());
                }
                let len = serde_bare::Uint(vm.len() as u64);
                let mut vl = serde_bare::to_vec::<serde_bare::Uint>(&len).unwrap();
                vl.append(&mut vm);
                return match self.connection.send(&vl).await {
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
        let r = TcpListener::create(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let connection = connection.unwrap();

        let mut s = Serializer::new(connection);
        match s.deserialize().await {
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

    async fn ok_sender(address: String) {
        let mut connection =
            TcpConnection::create(std::net::SocketAddr::from_str(&address).unwrap());
        let res = connection.connect().await;
        assert!(!res.is_err());

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
        let mut serializer = Serializer::new(connection);
        match serializer.serialize(m).await {
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
        let r = TcpListener::create(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let connection = connection.unwrap();

        let mut s = Serializer::new(connection);
        match s.deserialize().await {
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
        let mut connection = TcpConnection::create(std::net::SocketAddr::from_str(&a).unwrap());
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

    fn get_messages() -> [Message; 2] {
        let m1 = Message {
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
            message_body: MessageBody::Payload(vec![0xfu8; 32]),
        };
        let m2 = Message {
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
            message_body: MessageBody::Payload(vec![0x8u8; 32]),
        };
        [m1, m2]
    }

    async fn partial_message_listener(a: String) {
        let r = TcpListener::create(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let connection = connection.unwrap();
        let mut s = Serializer::new(connection);

        let messages = get_messages();

        // expect 2 messages, each with 32-byte payload
        for msg in messages.iter() {
            match s.deserialize().await {
                Ok(m) => {
                    assert_eq!(&m, msg);
                }
                Err(e) => {
                    panic!(format!("{:?}", e));
                }
            }
        }
    }

    async fn partial_message_sender(a: String) {
        let mut connection = TcpConnection::create(std::net::SocketAddr::from_str(&a).unwrap());
        let res = connection.connect().await;
        assert!(!res.is_err());

        let messages = get_messages();

        let mut vm1 = serde_bare::to_vec::<Message>(&messages[0]).unwrap();
        let len1 = serde_bare::Uint(vm1.len() as u64);
        let mut vl1 = serde_bare::to_vec::<serde_bare::Uint>(&len1).unwrap();
        vl1.append(&mut vm1);

        let mut vm2 = serde_bare::to_vec::<Message>(&messages[1]).unwrap();
        let len2 = serde_bare::Uint(vm2.len() as u64);
        let mut vl2 = serde_bare::to_vec::<serde_bare::Uint>(&len2).unwrap();
        vl2.append(&mut vm2);

        vl1.append(&mut vl2);

        connection.send(&vl1[0..16]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((1000.0) as u64)).await;
        connection.send(&vl1[16..58]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((1000.0) as u64)).await;
        connection.send(&vl1[58..]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((2000.0) as u64)).await;
    }

    async fn run_partial_message_test(address: String) {
        let a1 = address.clone();
        let j1 = task::spawn(async {
            let f = partial_message_listener(a1);
            f.await;
        });

        let a2 = address.clone();
        let j2 = task::spawn(async {
            let f = partial_message_sender(a2);
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
        });
    }

    #[test]
    fn big_message() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            println!("run_big_message_test starting...");
            run_big_message_test(String::from("127.0.0.1:4051")).await;
            println!("run_big_message_test done.");
        });
    }

    #[test]
    fn partial_message() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            println!("run_partial_message_test...");
            run_partial_message_test(String::from("127.0.0.1:4052")).await;
            println!("run_partial_message_test done.");
        });
    }
}
