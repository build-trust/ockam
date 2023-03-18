use ockam::{errcode::Origin, Error};
use ockam_core::Result;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

const AVAILABLE_LOCAL_PORTS_ADDR: &str = "127.0.0.1:0";

/// Helper function. Try to find numbers of available local UDP ports.
pub async fn available_local_ports(count: usize) -> Result<Vec<SocketAddr>> {
    let mut sockets = Vec::new();
    let mut addrs = Vec::new();

    for _ in 0..count {
        let s = UdpSocket::bind(AVAILABLE_LOCAL_PORTS_ADDR)
            .await
            .map_err(|e| Error::new_unknown(Origin::Unknown, e))?;
        let a = s
            .local_addr()
            .map_err(|e| Error::new_unknown(Origin::Unknown, e))?;

        addrs.push(a);

        // Keep sockets open until we are done asking for available ports
        sockets.push(s);
    }

    Ok(addrs)
}
