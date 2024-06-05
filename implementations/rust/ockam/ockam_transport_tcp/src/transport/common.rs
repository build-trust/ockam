use cfg_if::cfg_if;
use ockam_core::compat::net::SocketAddr;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use ockam_transport_core::{HostnamePort, TransportError};
use socket2::{SockRef, TcpKeepalive};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::{TlsConnector, TlsStream};
use tracing::{debug, instrument};

/// Connect to a socket address via a regular TcpStream
#[instrument(skip_all)]
pub(crate) async fn connect(socket_address: SocketAddr) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
    Ok(create_tcp_stream(socket_address).await?.into_split())
}

/// Create a TCP stream to a given socket address
pub(crate) async fn create_tcp_stream(socket_address: SocketAddr) -> Result<TcpStream> {
    debug!(addr = %socket_address, "Connecting");
    let connection = match TcpStream::connect(socket_address).await {
        Ok(c) => {
            debug!(addr = %socket_address, "Connected");
            c
        }
        Err(e) => {
            debug!(addr = %socket_address, err = %e, "Failed to connect");
            return Err(TransportError::from(e))?;
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

    Ok(connection)
}

/// Connect to a socket address via a TlsStream
#[allow(clippy::type_complexity)]
#[instrument(skip_all)]
pub(crate) async fn connect_tls(
    hostname_port: &HostnamePort,
) -> Result<(
    ReadHalf<TlsStream<TcpStream>>,
    WriteHalf<TlsStream<TcpStream>>,
)> {
    let socket_address = hostname_port.to_socket_addr()?;
    debug!(hostname_port = %hostname_port, addr = %socket_address, "Trying to connect using TLS");

    // create a tcp stream
    let connection = create_tcp_stream(socket_address).await?;

    // create a TLS connector
    let tls_connector = create_tls_connector().await?;

    // parse destination hostname
    let hostname = ServerName::try_from(hostname_port.hostname()).map_err(|e| {
        Error::new(
            Origin::Transport,
            Kind::Io,
            format!("Cannot create a ServerName from {hostname_port}: {e:?}"),
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
                format!("Cannot connect using TLS to {hostname_port}: {e:?}"),
            )
        })?;
    debug!("Connected using TLS to {hostname_port}");

    Ok(tokio::io::split(TlsStream::from(client_tls_stream)))
}

/// Create a TLS connector using the system certificates
pub(crate) async fn create_tls_connector() -> Result<TlsConnector> {
    let certificates = rustls_native_certs::load_native_certs().map_err(|e| {
        Error::new(
            Origin::Transport,
            Kind::Io,
            format!("Cannot load the native certificates: {e:?}"),
        )
    })?;
    debug!("there are {} certificates", certificates.len());

    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.add_parsable_certificates(certificates);

    let config = ClientConfig::builder()
        .with_root_certificates(root_cert_store)
        .with_no_client_auth();

    Ok(TlsConnector::from(Arc::new(config)))
}
