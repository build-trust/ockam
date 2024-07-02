pub use tokio::sync::Mutex;
pub use tokio::sync::RwLock;

use ockam_transport_core::{parse_socket_addr, TransportError};
use std::net::SocketAddr;
use tokio::net::lookup_host;

/// Asynchronously resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
pub async fn resolve_peer(peer: impl ToString) -> ockam_core::Result<SocketAddr> {
    let peer = peer.to_string();
    // Try to parse as SocketAddr
    if let Ok(p) = parse_socket_addr(&peer) {
        return Ok(p);
    }

    // Try to resolve hostname
    match lookup_host(peer.clone()).await {
        Ok(mut iter) => {
            // Prefer ip4
            if let Some(p) = iter.find(|x| x.is_ipv4()) {
                return Ok(p);
            }
            if let Some(p) = iter.find(|x| x.is_ipv6()) {
                return Ok(p);
            }
            Err(TransportError::InvalidAddress(format!(
                "cannot resolve address: {peer}. No IP4 or IP6 address found."
            )))?
        }
        Err(e) => Err(TransportError::InvalidAddress(format!(
            "cannot resolve address: {peer}: {e:?}"
        )))?,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ockam_transport_core::HostnamePort;

    #[tokio::test]
    async fn test_hostname_port() -> ockam_core::Result<()> {
        let socket_addr = resolve_peer("76.76.21.21:8080".to_string()).await.unwrap();
        let actual = HostnamePort::try_from(socket_addr).ok();
        assert_eq!(actual, Some(HostnamePort::new("76.76.21.21", 8080)));

        Ok(())
    }
}
