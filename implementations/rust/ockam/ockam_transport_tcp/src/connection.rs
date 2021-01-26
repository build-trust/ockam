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
    stream: Option<Arc<Mutex<TcpStream>>>,
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
            stream: Some(Arc::new(Mutex::new(stream))),
        }))
    }
}

#[async_trait]
impl Connection for TcpConnection {
    async fn connect(&mut self) -> Result<(), String> {
        self.stream = Some(Arc::new(Mutex::new(
            TcpStream::connect(self.remote_address).await.unwrap(),
        )));
        Ok(())
    }

    async fn send(&mut self, buff: &[u8]) -> Result<usize, String> {
        let mut i = 0;
        return if let Some(stream) = &self.stream {
            let stream = stream.lock().await;
            loop {
                if std::result::Result::is_err(&stream.writable().await) {
                    return Err("Can't send, check connection".into());
                }
                match stream.try_write(&buff[i..]) {
                    Ok(n) if n == buff.len() => {
                        return Ok(n);
                    }
                    Ok(n) => {
                        i = i + n;
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
            let stream = stream.lock().await;
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
                        assert!(false);
                        return Err("receive failed".into());
                    }
                }
            }
        } else {
            Err("not connected".into())
        }
    }
}
