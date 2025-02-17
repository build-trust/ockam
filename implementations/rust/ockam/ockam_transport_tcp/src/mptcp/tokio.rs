use std::{
    future::{poll_fn, Future},
    io,
    net::SocketAddr,
};

use tokio::net::{lookup_host, TcpListener, TcpStream, ToSocketAddrs};

use super::sys::MptcpSocketBuilder;

// FIXME
async fn resolve_each_addr<A: ToSocketAddrs, F, Fut, T>(addr: &A, mut f: F) -> io::Result<T>
where
    F: FnMut(SocketAddr) -> Fut,
    Fut: Future<Output = io::Result<T>>,
{
    let addrs = lookup_host(addr).await?;
    let mut last_err = None;
    for addr in addrs {
        match f(addr).await {
            Ok(l) => return Ok(l),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "could not resolve to any address",
        )
    }))
}

/// Connect using MPTCP
pub async fn connect_mptcp<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
    resolve_each_addr(&addr, |addr| async move {
        let sock = MptcpSocketBuilder::new_for_addr(addr)?
            .set_nonblocking()?
            .connect(addr)
            .and_then(|sock| TcpStream::from_std(sock.into()))?;
        // Wait for the socket to be writable
        poll_fn(|cx| sock.poll_write_ready(cx)).await?;
        Ok(sock)
    })
    .await
}

/// Bind using MPTCP
pub async fn bind_mptcp(addr: SocketAddr) -> io::Result<TcpListener> {
    let socket = MptcpSocketBuilder::new_for_addr(addr)?
        .set_nonblocking()?
        .bind(addr)?;

    TcpListener::from_std(socket.into())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use crate::mptcp::tokio::resolve_each_addr;
    use crate::mptcp::{bind_mptcp, connect_mptcp, is_mptcp_enabled};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

    #[tokio::test]
    async fn test_resolve_each_addr() {
        let addr = "127.0.0.1:80";
        let result = resolve_each_addr(&addr, |addr| async move {
            assert_eq!(addr.port(), 80);
            assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
            Ok(())
        })
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_each_addr_error() {
        let addr = "thisisanerror";
        let result = resolve_each_addr(&addr, |_| async { Ok(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mptcp_socket() {
        let mptcp_enabled = is_mptcp_enabled();

        let listener = bind_mptcp(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            0,
        )))
        .await;
        if mptcp_enabled {
            assert!(listener.is_ok());
        } else {
            assert!(listener.is_err());
        }

        let local_addr = listener.unwrap().local_addr().unwrap();

        let stream = connect_mptcp(local_addr).await;
        if mptcp_enabled {
            assert!(stream.is_ok());
        } else {
            assert!(stream.is_err());
        }
    }
}
