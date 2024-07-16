use ockam_core::compat::sync::Arc;
use std::io;
use std::net::SocketAddr;
use tokio::net::{ToSocketAddrs, UdpSocket};

pub fn split_socket(socket: UdpSocket) -> (UdpSocketRead, UdpSocketWrite) {
    let socket = Arc::new(socket);

    (UdpSocketRead(socket.clone()), UdpSocketWrite(socket))
}

#[derive(Debug, Clone)]
pub struct UdpSocketRead(Arc<UdpSocket>);

impl UdpSocketRead {
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.0.recv_from(buf).await
    }
}

#[derive(Debug, Clone)]
pub struct UdpSocketWrite(Arc<UdpSocket>);

impl UdpSocketWrite {
    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], target: A) -> io::Result<usize> {
        self.0.send_to(buf, target).await
    }
}
