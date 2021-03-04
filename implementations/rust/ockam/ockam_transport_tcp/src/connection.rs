use crate::error::TransportError;
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_router::message::{RouterAddress, TransportMessage};
use ockam_router::{
    print_route, RouteTransportMessage, RouterError, ROUTER_ADDRESS, ROUTER_ADDRESS_TYPE_TCP,
};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::io;
use tokio::net::TcpStream;

// ToDo - revisit these values
const MAX_MESSAGE_SIZE: usize = 2048;
const DEFAULT_TCP_ADDRESS: &str = "127.0.0.1:4050";

pub struct TcpConnection {
    remote_address: std::net::SocketAddr,
    local_address: std::net::SocketAddr,
    _blocking: bool,
    message_buff: Vec<u8>,
    message_length: usize,
    stream: Option<tokio::net::TcpStream>,
}

impl TcpConnection {
    pub fn create(remote_address: SocketAddr) -> Box<TcpConnection> {
        let c = Box::new(TcpConnection {
            remote_address,
            local_address: SocketAddr::from_str(&DEFAULT_TCP_ADDRESS).unwrap(),
            _blocking: true,
            message_buff: vec![],
            message_length: 0,
            stream: None,
        });
        c
    }

    pub async fn new_from_stream(stream: TcpStream) -> Result<Box<TcpConnection>> {
        match stream.peer_addr() {
            Ok(peer) => {
                let c = Box::new(TcpConnection {
                    remote_address: peer,
                    local_address: SocketAddr::from_str(&DEFAULT_TCP_ADDRESS).unwrap(),
                    _blocking: true,
                    message_buff: vec![],
                    message_length: 0,
                    stream: Some(stream),
                });
                Ok(c)
            }
            Err(_) => Err(TransportError::PeerNotFound.into()),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        match self.stream {
            Some(_) => Ok(()), //Err(TransportError::AlreadyConnected.into()),
            None => match TcpStream::connect(&self.remote_address).await {
                Ok(s) => {
                    self.stream = Some(s);
                    Ok(())
                }
                Err(_) => Err(TransportError::ConnectFailed.into()),
            },
        }
    }

    pub async fn send(&mut self, buff: &[u8]) -> Result<usize> {
        let mut i = 0;
        return if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.writable().await) {
                    return Err(TransportError::CheckConnection.into());
                }
                match stream.try_write(&buff[i..]) {
                    Ok(n) if n == buff.len() => {
                        return Ok(n);
                    }
                    Ok(n) => {
                        i += n;
                        continue;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        return Err(TransportError::CheckConnection.into());
                    }
                    Err(_) => {
                        return Err(TransportError::CheckConnection.into());
                    }
                }
            }
        } else {
            Err(TransportError::NotConnected.into())
        };
    }

    pub async fn receive(&mut self, buff: &mut [u8]) -> Result<usize> {
        if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.readable().await) {
                    return Err(TransportError::CheckConnection.into());
                }
                match stream.try_read(buff) {
                    Ok(n) => {
                        return if 0 == n {
                            Err(TransportError::ConnectionClosed.into())
                        } else {
                            Ok(n)
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    _ => {
                        return Err(TransportError::ReceiveFailed.into());
                    }
                }
            }
        } else {
            Err(TransportError::CheckConnection.into())
        }
    }

    pub async fn send_message(&mut self, mut msg: TransportMessage) -> Result<usize> {
        if !msg.onward_route.addrs.is_empty() {
            if msg.onward_route.addrs[0] == self.get_remote_address() {
                msg.onward_route.addrs.remove(0);
            }
        }
        msg.return_route
            .addrs
            .insert(0, self.get_local_address().clone());
        return match serde_bare::to_vec::<TransportMessage>(&msg) {
            Ok(mut msg_vec) => {
                if msg_vec.len() > MAX_MESSAGE_SIZE - 2 {
                    return Err(TransportError::IllFormedMessage.into());
                }
                let len = msg_vec.len() as u16;
                let mut msg_len_vec = len.to_be_bytes().to_vec();
                msg_len_vec.append(&mut msg_vec);
                return self.send(&msg_len_vec).await;
            }
            Err(_) => Err(TransportError::IllFormedMessage.into()),
        };
    }

    pub async fn receive_message(&mut self) -> Result<TransportMessage> {
        loop {
            let mut recv_buff = [0u8; MAX_MESSAGE_SIZE];

            // first see if we have a complete message from the last call
            // if not, read additional bytes
            if self.message_buff.len() <= self.message_length as usize {
                let bytes_received = self.receive(&mut recv_buff).await?;
                self.message_buff
                    .append(&mut recv_buff[0..bytes_received].to_vec());
            }

            if self.message_length == 0 {
                let (len, _) = recv_buff.split_at(2);
                self.message_length = u16::from_be_bytes(len.try_into().unwrap()) as usize;
                self.message_buff.remove(0);
                self.message_buff.remove(0);
            }

            if self.message_length as usize <= self.message_buff.len() {
                // we have a complete message
                println!(
                    "connection: complete message, len {} self.message_buff len {}: ",
                    self.message_length,
                    self.message_buff.len()
                );
                return match serde_bare::from_slice::<TransportMessage>(&self.message_buff) {
                    Ok(mut m) => {
                        println!(
                            "connection: message received with payload len {} and onward route:",
                            m.payload.len()
                        );
                        print_route(&m.onward_route.addrs);
                        // scoot any remaining bytes to the beginning of the buffer
                        for i in 0..self.message_buff.len() - self.message_length {
                            self.message_buff[i] = self.message_buff[i + self.message_length];
                        }
                        self.message_buff
                            .truncate(self.message_buff.len() - self.message_length);
                        self.message_length = 0;
                        if self.message_buff.len() >= 2 {
                            let len = &self.message_buff[..2];
                            self.message_length =
                                u16::from_be_bytes(len.try_into().unwrap()) as usize;
                            self.message_buff.remove(0);
                            self.message_buff.remove(0);
                        }

                        // if first address in onward route is ours, remove it
                        if !m.onward_route.addrs.is_empty()
                            && m.onward_route.addrs[0] == self.get_local_address()
                        {
                            m.onward_route.addrs.remove(0);
                        }

                        if !m.onward_route.addrs.is_empty()
                            && m.onward_route.addrs[0].address_type == ROUTER_ADDRESS_TYPE_TCP
                        {
                            let router_addr =
                                serde_bare::to_vec::<SocketAddr>(&self.local_address).unwrap();
                            m.return_route.addrs.push(RouterAddress {
                                address_type: ROUTER_ADDRESS_TYPE_TCP,
                                address: router_addr,
                            });
                            self.send_message(m).await?;
                            continue;
                        }
                        Ok(m)
                    }
                    Err(_) => Err(TransportError::IllFormedMessage.into()),
                };
            }
        }
    }

    pub fn get_local_address(&self) -> RouterAddress {
        return match &self.stream {
            Some(stream) => {
                let ra = serde_bare::to_vec::<SocketAddr>(&stream.local_addr().unwrap()).unwrap();
                RouterAddress {
                    address_type: ROUTER_ADDRESS_TYPE_TCP,
                    address: ra,
                }
            }
            None => RouterAddress {
                address_type: ROUTER_ADDRESS_TYPE_TCP,
                address: vec![],
            },
        };
    }

    pub fn get_remote_address(&self) -> RouterAddress {
        let ra = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_TCP,
            address: ra,
        }
    }

    pub fn get_worker_address(&self) -> Address {
        let addr = self.get_remote_address();
        Address::from(serde_bare::to_vec::<RouterAddress>(&addr).unwrap())
    }

    pub fn get_routeable_address(&self) -> Vec<u8> {
        let mut v = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        v.insert(0, ROUTER_ADDRESS_TYPE_TCP);
        v
    }

    pub fn get_router_address(&self) -> RouterAddress {
        let v = serde_bare::to_vec::<SocketAddr>(&self.remote_address).unwrap();
        RouterAddress {
            address_type: 1,
            address: v,
        }
    }

    pub fn get_remote_socket(&self) -> SocketAddr {
        return match &self.stream {
            Some(s) => s.peer_addr().unwrap(),
            None => SocketAddr::from_str("0.0.0.0:00").unwrap(),
        };
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum TcpWorkerMessage {
    SendMessage(TransportMessage),
    Receive,
}

#[async_trait]
impl Worker for Box<TcpConnection> {
    type Message = TcpWorkerMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        println!("in tcp_worker");
        return match msg {
            TcpWorkerMessage::SendMessage(m) => {
                if self.send_message(m).await.is_err() {
                    Err(TransportError::ConnectionClosed.into())
                } else {
                    Ok(())
                }
            }
            TcpWorkerMessage::Receive => {
                println!("tcp_worker receive pending...");
                return if let Ok(msg) = self.receive_message().await {
                    println!(
                        "tcp_worker got message, payload length {}",
                        msg.payload.len()
                    );
                    if msg.onward_route.addrs.is_empty() {
                        return Err(RouterError::NoRoute.into());
                    }
                    return match ctx
                        .send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(msg))
                        .await
                    {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e),
                    };
                } else {
                    Ok(())
                };
            }
        };
    }
}

#[cfg(test)]
mod test {
    use crate::connection::TcpConnection;
    use crate::listener::TcpListener;
    use ockam_router::{
        Route, RouterAddress, TransportMessage, ROUTER_ADDRESS_TYPE_LOCAL, ROUTER_ADDRESS_TYPE_TCP,
    };
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::runtime::Builder;
    use tokio::task;
    use tokio::time::Duration;

    async fn client_worker(address: String) {
        let mut connection =
            TcpConnection::create(std::net::SocketAddr::from_str(&address).unwrap());
        let r = connection.connect().await;
        assert!(!r.is_err());
        for _i in 0u16..5 {
            let r = connection.send(b"ping").await;
            assert!(r.is_ok());
            let bytes = r.unwrap();
            assert_eq!(bytes, 4);

            let mut buff: [u8; 32] = [0; 32];
            let r = connection.receive(&mut buff).await;
            assert!(r.is_ok());
            let bytes = r.unwrap();
            assert_eq!(bytes, 4);
            assert_eq!(&buff[0..4], b"pong");
        }
        return;
    }

    async fn listen_worker(address: String) {
        {
            let res = TcpListener::create(std::net::SocketAddr::from_str(&address).unwrap()).await;
            assert!(res.is_ok());

            let mut listener = res.unwrap();
            let connection = listener.accept().await;
            assert!(connection.is_ok());

            let mut connection = connection.unwrap();
            for _i in 0u16..5 {
                let mut buff: [u8; 32] = [0; 32];
                let r = connection.receive(&mut buff).await;
                assert!(r.is_ok());
                assert_eq!(r.unwrap(), 4);
                assert_eq!(&buff[0..4], b"ping");

                let r = connection.send(b"pong").await;
                assert!(r.is_ok());
                assert_eq!(r.unwrap(), 4);
            }
        }
    }

    async fn run_test(address: String) {
        let a1 = address.clone();
        let j1 = task::spawn(async {
            let f = listen_worker(a1);
            f.await;
        });

        let a2 = address.clone();
        let j2 = task::spawn(async {
            let f = client_worker(a2);
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
    pub fn ping_pong_multi_thread() {
        let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

        runtime.block_on(async {
            run_test(String::from("127.0.0.1:4053")).await;
        });
    }

    #[test]
    pub fn ping_pong_single_thread() {
        let runtime = Builder::new_current_thread().enable_io().build().unwrap();

        runtime.block_on(async {
            run_test(String::from("127.0.0.1:4054")).await;
        });
    }

    async fn ok_listener(a: String) {
        let r = TcpListener::create(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let mut connection = connection.unwrap();

        let sock_addr = SocketAddr::from_str(&a).unwrap();
        let sock_addr_as_vec = serde_bare::to_vec::<SocketAddr>(&sock_addr).unwrap();
        let router_sock_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_TCP,
            address: sock_addr_as_vec,
        };
        let router_local_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_LOCAL,
            address: b"0123".to_vec(),
        };
        let remote_transport_addr = connection.get_remote_address();

        match connection.receive_message().await {
            Ok(m) => {
                assert_eq!(
                    m,
                    TransportMessage {
                        version: 1,
                        onward_route: Route {
                            addrs: vec![router_local_addr.clone()],
                        },
                        return_route: Route {
                            addrs: vec![remote_transport_addr, router_sock_addr, router_local_addr],
                        },
                        payload: vec![0u8],
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

        let sock_addr = SocketAddr::from_str(&address).unwrap();
        let sock_addr_as_vec = serde_bare::to_vec::<SocketAddr>(&sock_addr).unwrap();
        let router_sock_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_TCP,
            address: sock_addr_as_vec,
        };
        let router_local_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_LOCAL,
            address: b"0123".to_vec(),
        };

        let m = TransportMessage {
            version: 1,
            onward_route: Route {
                addrs: vec![router_sock_addr.clone(), router_local_addr.clone()],
            },
            return_route: Route {
                addrs: vec![router_sock_addr, router_local_addr],
            },
            payload: vec![0],
        };
        match connection.send_message(m).await {
            Ok(_) => {}
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
        let mut connection = connection.unwrap();

        let sock_addr = SocketAddr::from_str(&a).unwrap();
        let sock_addr_as_vec = serde_bare::to_vec::<SocketAddr>(&sock_addr).unwrap();
        let router_local_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_LOCAL,
            address: b"0123".to_vec(),
        };

        match connection.receive_message().await {
            Ok(m) => {
                println!("{:?} {:?}", m.onward_route, m.return_route);
                assert_eq!(
                    m,
                    TransportMessage {
                        version: 1,
                        onward_route: Route {
                            addrs: vec![router_local_addr.clone()]
                        },
                        return_route: Route {
                            addrs: vec![router_local_addr]
                        },
                        payload: vec![0xfu8; 1024]
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

        let router_local_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_LOCAL,
            address: b"0123".to_vec(),
        };

        let m = TransportMessage {
            version: 1,
            onward_route: Route {
                addrs: vec![router_local_addr.clone()],
            },
            return_route: Route {
                addrs: vec![router_local_addr],
            },
            payload: vec![0xfu8; 1024],
        };
        let mut vm = serde_bare::to_vec::<TransportMessage>(&m).unwrap();
        let len = vm.len() as u16;
        let mut vl = len.to_be_bytes().to_vec();
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

    fn get_messages() -> [TransportMessage; 2] {
        let router_local_addr = RouterAddress {
            address_type: ROUTER_ADDRESS_TYPE_LOCAL,
            address: b"0123".to_vec(),
        };
        let m1 = TransportMessage {
            version: 1,
            onward_route: Route {
                addrs: vec![router_local_addr.clone()],
            },
            return_route: Route {
                addrs: vec![router_local_addr.clone()],
            },
            payload: vec![0xfu8; 32],
        };
        let m2 = m1.clone();
        [m1, m2]
    }

    async fn partial_message_listener(a: String) {
        let r = TcpListener::create(std::net::SocketAddr::from_str(&a).unwrap()).await;
        assert!(r.is_ok());

        let mut listener = r.unwrap();
        let connection = listener.accept().await;
        assert!(connection.is_ok());
        let mut connection = connection.unwrap();

        let messages = get_messages();

        // expect 2 messages, each with 32-byte payload
        for msg in messages.iter() {
            let msg = msg.clone();
            match connection.receive_message().await {
                Ok(m) => {
                    println!("m  : {:?}", m);
                    println!("msg: {:?}", msg);
                    assert_eq!(m, msg);
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

        let mut vm1 = serde_bare::to_vec::<TransportMessage>(&messages[0]).unwrap();
        let len1 = vm1.len() as u16;
        let mut vl1 = len1.to_be_bytes().to_vec();
        vl1.append(&mut vm1);

        let mut vm2 = serde_bare::to_vec::<TransportMessage>(&messages[1]).unwrap();
        let len2 = vm2.len() as u16;
        let mut vl2 = len2.to_be_bytes().to_vec();
        vl2.append(&mut vm2);

        vl1.append(&mut vl2);

        connection.send(&vl1[0..16]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((100.0) as u64)).await;
        connection.send(&vl1[16..58]).await.unwrap();
        tokio::time::sleep(Duration::from_millis((100.0) as u64)).await;
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
