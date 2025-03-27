use std::{future::poll_fn, io};
use tokio::net::{lookup_host, TcpListener, TcpStream, ToSocketAddrs};

use super::sys::MptcpSocketBuilder;

/// Connect using MPTCP
pub async fn connect_mptcp(addr: impl ToSocketAddrs) -> io::Result<TcpStream> {
    let addrs = lookup_host(addr).await?;

    let mut last_err = None;

    for addr in addrs {
        let stream = {
            let sock = MptcpSocketBuilder::new_for_addr(addr)?
                .set_nonblocking()?
                .connect(addr)
                .and_then(|sock| TcpStream::from_std(sock.into()))?;

            // Once we've connected, wait for the stream to be writable as
            // that's when the actual connection has been initiated. Once we're
            // writable we check for `take_socket_error` to see if the connect
            // actually hit an error or not.
            //
            // If all that succeeded then we ship everything on up.
            poll_fn(|cx| sock.poll_write_ready(cx)).await?;

            if let Some(e) = sock.take_error()? {
                return Err(e);
            }

            Ok(sock)
        };

        match stream {
            Ok(stream) => return Ok(stream),
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

/// Bind using MPTCP
pub async fn bind_mptcp(addr: impl ToSocketAddrs) -> io::Result<TcpListener> {
    let addrs = lookup_host(addr).await?;

    let mut last_err = None;

    for addr in addrs {
        let listener = {
            let builder = MptcpSocketBuilder::new_for_addr(addr)?.set_nonblocking()?;

            #[cfg(not(windows))]
            let builder = builder.set_reuse()?;

            let socket = builder.bind(addr)?;

            TcpListener::from_std(socket.into())
        };

        match listener {
            Ok(listener) => return Ok(listener),
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

#[cfg(test)]
mod tests {
    use crate::mptcp::{bind_mptcp, connect_mptcp, is_mptcp_enabled};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

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
            return;
        }

        let listener = listener.unwrap();
        let local_addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                listener.accept().await.unwrap();
            }
        });

        let stream = connect_mptcp(local_addr).await;

        if let Err(err) = stream {
            panic!("{}", err);
        }
    }
}
