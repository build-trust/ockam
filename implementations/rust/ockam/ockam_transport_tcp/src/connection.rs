use crate::traits::Connection;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::result::Result;
use std::sync::Arc;
use tokio::io;
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct TcpConnection {
    remote_address: std::net::SocketAddr,
    _blocking: bool,
    stream: Option<TcpStream>,
}

impl TcpConnection {
    pub fn new(remote_address: SocketAddr) -> Arc<Mutex<dyn Connection + Send>> {
        Arc::new(Mutex::new(TcpConnection {
            remote_address,
            _blocking: true,
            stream: None,
        }))
    }
    pub async fn new_from_stream(stream: TcpStream) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(TcpConnection {
            remote_address: stream.peer_addr().unwrap(),
            _blocking: true,
            stream: Some(stream),
        }))
    }
}

#[async_trait]
impl Connection for TcpConnection {
    async fn connect(&mut self) -> Result<(), String> {
        self.stream = Some(TcpStream::connect(self.remote_address).await.unwrap());
        Ok(())
    }

    async fn send(&mut self, buff: &[u8]) -> Result<usize, String> {
        let mut i = 0;
        return if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.writable().await) {
                    return Err("Can't send, check connection".into());
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
                        continue;
                    }
                    Err(_) => {
                        return Err("write error".into());
                    }
                }
            }
        } else {
            Err("not connected".into())
        };
    }

    async fn receive(&mut self, buff: &mut [u8]) -> Result<usize, String> {
        if let Some(stream) = &self.stream {
            loop {
                if std::result::Result::is_err(&stream.readable().await) {
                    return Err("can't receive, check connection".into());
                }
                match stream.try_read(buff) {
                    Ok(n) => {
                        return Ok(n);
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        continue;
                    }
                    _ => {
                        return Err("receive failed".into());
                    }
                }
            }
        } else {
            Err("not connected".into())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::connection::TcpConnection;
    use crate::listener::OckamTcpListener;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::runtime::Builder;
    use tokio::task;

    async fn client_worker(address: String) {
        let connection = TcpConnection::new(SocketAddr::from_str(&address).unwrap());
        let mut connection = connection.lock().await;
        let r = connection.connect().await;
        assert!(!r.is_err());
        for _i in 0..5 {
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
            let r = OckamTcpListener::new(SocketAddr::from_str(&address).unwrap()).await;
            assert!(r.is_ok());

            let listener = r.unwrap();
            let mut listener = listener.lock().await;
            let connection = listener.accept().await;
            assert!(connection.is_ok());

            let connection = connection.unwrap();
            let mut connection = connection.lock().await;
            for _i in 0..5 {
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
            return;
        });

        let a2 = address.clone();
        let j2 = task::spawn(async {
            let f = client_worker(a2);
            f.await;
            return;
        });
        let (r1, r2) = tokio::join!(j1, j2);
        if r1.is_err() {
            println!("{:?}", r1);
            assert!(false);
        }
        if r2.is_err() {
            println!("{:?}", r2);
            assert!(false);
        }
    }

    #[test]
    pub fn ping_pong_multi_thread() {
        let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

        runtime.block_on(async {
            run_test(String::from("127.0.0.1:4050")).await;
        });
    }

    #[test]
    pub fn ping_pong_single_thread() {
        let runtime = Builder::new_current_thread().enable_io().build().unwrap();

        runtime.block_on(async {
            run_test(String::from("127.0.0.1:4051")).await;
        });
    }
}
