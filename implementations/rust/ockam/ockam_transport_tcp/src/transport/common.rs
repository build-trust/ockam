use crate::mptcp::{bind_mptcp, connect_mptcp};
use cfg_if::cfg_if;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_transport_core::{HostnamePort, TransportError};
use socket2::{SockRef, TcpKeepalive};
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, RawFd};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::{TlsConnector, TlsStream};
use tracing::{debug, instrument, Level};

pub fn set_socket_buffer_size(socket: RawFd, buffer_size: usize) -> Result<()> {
    let buffer_size = buffer_size as nix::libc::c_int;

    let res = unsafe {
        #[allow(trivial_casts)]
        nix::libc::setsockopt(
            socket.as_raw_fd(),
            nix::libc::SOL_SOCKET,
            nix::libc::SO_RCVBUF,
            (&buffer_size as *const nix::libc::c_int) as *const nix::libc::c_void,
            size_of::<nix::libc::c_int>() as nix::libc::socklen_t,
        )
    };

    if res < 0 {
        return Err(TransportError::SockOpt("TCP portal receive buffer error".to_string()).into());
    }

    let res = unsafe {
        #[allow(trivial_casts)]
        nix::libc::setsockopt(
            socket.as_raw_fd(),
            nix::libc::SOL_SOCKET,
            nix::libc::SO_SNDBUF,
            (&buffer_size as *const nix::libc::c_int) as *const nix::libc::c_void,
            size_of::<nix::libc::c_int>() as nix::libc::socklen_t,
        )
    };

    if res < 0 {
        return Err(TransportError::SockOpt("TCP portal send buffer error".to_string()).into());
    }

    Ok(())
}

pub(crate) async fn bind_tcp_listener(
    at: SocketAddr,
    enable_mptcp: bool,
    buffer_size: Option<usize>,
) -> Result<TcpListener> {
    let listener = if !enable_mptcp {
        TcpListener::bind(&at).await.map_err(TransportError::from)?
    } else {
        bind_mptcp(at).await.map_err(TransportError::from)?
    };

    if let Some(buffer_size) = buffer_size {
        set_socket_buffer_size(listener.as_raw_fd(), buffer_size)?;
    }

    Ok(listener)
}

async fn create_tcp_stream(
    to: &HostnamePort,
    enable_mptcp: bool,
    buffer_size: Option<usize>,
) -> Result<TcpStream> {
    let stream = if !enable_mptcp {
        TcpStream::connect(to.to_string())
            .await
            .map_err(TransportError::from)?
    } else {
        connect_mptcp(to.to_string())
            .await
            .map_err(TransportError::from)?
    };

    if let Some(buffer_size) = buffer_size {
        set_socket_buffer_size(stream.as_raw_fd(), buffer_size)?;
    }

    Ok(stream)
}

async fn create_tcp_stream_timeout(
    to: &HostnamePort,
    enable_mptcp: bool,
    timeout: Option<Duration>,
    buffer_size: Option<usize>,
) -> Result<TcpStream> {
    match timeout {
        Some(timeout) => {
            match tokio::time::timeout(timeout, create_tcp_stream(to, enable_mptcp, buffer_size))
                .await
            {
                Ok(result) => result,
                Err(_) => {
                    debug!(addr = %to, timeout = %timeout.as_secs(),  "Timeout");
                    Err(TransportError::ConnectionTimeout)?
                }
            }
        }
        None => create_tcp_stream(to, enable_mptcp, buffer_size).await,
    }
}

/// Connect to a socket address via a regular TcpStream
#[instrument(skip_all, level = Level::TRACE)]
pub(crate) async fn connect_tcp(
    to: &HostnamePort,
    enable_mptcp: bool,
    enable_nagle: bool,
    timeout: Option<Duration>,
    buffer_size: Option<usize>,
) -> Result<TcpStream> {
    debug!(addr = %to, "Connecting");

    let result = create_tcp_stream_timeout(to, enable_mptcp, timeout, buffer_size).await;

    let connection = match result {
        Ok(c) => {
            debug!(addr = %to, "Connected");
            c
        }
        Err(e) => {
            debug!(addr = %to, err = %e, "Failed to connect");
            return Err(e);
        }
    };

    let mut keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(300))
        .with_interval(Duration::from_secs(75));

    cfg_if! {
        if #[cfg(unix)] {
           keepalive = keepalive.with_retries(2);
        }
    }

    let socket = SockRef::from(&connection);
    socket
        .set_tcp_keepalive(&keepalive)
        .map_err(TransportError::from)?;

    socket
        .set_nodelay(!enable_nagle)
        .map_err(TransportError::from)?;

    Ok(connection)
}

/// Connect to a socket address via a TlsStream
#[allow(clippy::type_complexity)]
#[instrument(skip_all, level = Level::TRACE)]
pub(crate) async fn connect_tls(
    to: &HostnamePort,
    enable_mptcp: bool,
    enable_nagle: bool,
    buffer_size: Option<usize>,
) -> Result<(
    ReadHalf<TlsStream<TcpStream>>,
    WriteHalf<TlsStream<TcpStream>>,
)> {
    debug!(to = %to, "Trying to connect using TLS");

    // create a tcp stream
    let connection = connect_tcp(to, enable_mptcp, enable_nagle, None, buffer_size).await?;

    // create a TLS connector
    let tls_connector = create_tls_connector().await?;

    // parse destination hostname
    let hostname = ServerName::try_from(to.hostname()).map_err(|e| {
        Error::new(
            Origin::Transport,
            Kind::Io,
            format!("Cannot create a ServerName from {to}: {e:?}"),
        )
    })?;

    // Connect using TLS over TCP
    let client_tls_stream = tls_connector
        .connect(hostname, connection)
        .await
        .map_err(|e| {
            Error::new(
                Origin::Transport,
                Kind::Io,
                format!("Cannot connect using TLS to {to}: {e:?}"),
            )
        })?;
    debug!("Connected using TLS to {to}");

    Ok(tokio::io::split(TlsStream::from(client_tls_stream)))
}

/// Create a TLS connector using the system certificates
pub(crate) async fn create_tls_connector() -> Result<TlsConnector> {
    let certificates = rustls_native_certs::load_native_certs();

    if let Some(e) = certificates.errors.first() {
        return Err(Error::new(
            Origin::Transport,
            Kind::Io,
            format!("Cannot load the native certificates: {e:?}"),
        ));
    };

    let certificates = certificates.certs;

    debug!("there are {} certificates", certificates.len());

    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.add_parsable_certificates(certificates);

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    Ok(TlsConnector::from(Arc::new(config)))
}
