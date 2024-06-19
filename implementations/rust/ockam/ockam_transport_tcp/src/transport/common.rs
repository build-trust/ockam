use crate::TcpConnectionMode;
use cfg_if::cfg_if;
use core::fmt;
use core::fmt::Formatter;
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Error, Result};
use ockam_node::{Context, HostnamePort};
use ockam_transport_core::TransportError;
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

/// Result of [`TcpTransport::connect`] call.
#[derive(Clone, Debug)]
pub struct TcpConnection {
    sender_address: Address,
    receiver_address: Address,
    socket_address: SocketAddr,
    mode: TcpConnectionMode,
    flow_control_id: FlowControlId,
}

impl fmt::Display for TcpConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Socket: {}, Worker: {}, Processor: {}, FlowId: {}",
            self.socket_address, self.sender_address, self.receiver_address, self.flow_control_id
        )
    }
}

impl From<TcpConnection> for Address {
    fn from(value: TcpConnection) -> Self {
        value.sender_address
    }
}

impl TcpConnection {
    /// Constructor
    pub fn new(
        sender_address: Address,
        receiver_address: Address,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            sender_address,
            receiver_address,
            socket_address,
            mode,
            flow_control_id,
        }
    }
    /// Stops the [`TcpConnection`], this method must be called to avoid
    /// leakage of the connection.
    /// Simply dropping this object won't close the connection
    pub async fn stop(&self, context: &Context) -> Result<()> {
        context.stop_worker(self.sender_address.clone()).await
    }
    /// Corresponding [`TcpSendWorker`](super::workers::TcpSendWorker) [`Address`] that can be used
    /// in a route to send messages to the other side of the TCP connection
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }
    /// Corresponding [`TcpReceiveProcessor`](super::workers::TcpRecvProcessor) [`Address`]
    pub fn receiver_address(&self) -> &Address {
        &self.receiver_address
    }
    /// Corresponding [`SocketAddr`]
    pub fn socket_address(&self) -> &SocketAddr {
        &self.socket_address
    }
    /// Generated fresh random [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// Corresponding [`TcpConnectionMode`]
    pub fn mode(&self) -> TcpConnectionMode {
        self.mode
    }
}

/// Result of [`TcpTransport::listen`] call.
#[derive(Clone, Debug)]
pub struct TcpListener {
    processor_address: Address,
    socket_address: SocketAddr,
    flow_control_id: FlowControlId,
}

impl fmt::Display for TcpListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Socket: {}, Processor: {}, FlowId: {}",
            self.socket_address, self.processor_address, self.flow_control_id
        )
    }
}

impl TcpListener {
    /// Constructor
    pub fn new(
        processor_address: Address,
        socket_address: SocketAddr,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            processor_address,
            socket_address,
            flow_control_id,
        }
    }
    /// Corresponding Worker [`Address`] that can be used to stop the Listener
    pub fn processor_address(&self) -> &Address {
        &self.processor_address
    }
    /// Corresponding [`SocketAddr`]
    pub fn socket_address(&self) -> &SocketAddr {
        &self.socket_address
    }
    /// Corresponding [`SocketAddr`] in String format
    pub fn socket_string(&self) -> String {
        self.socket_address.to_string()
    }
    /// Generated fresh random [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

/// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
pub fn resolve_peer(peer: String) -> Result<SocketAddr> {
    // Try to parse as SocketAddr
    if let Ok(p) = parse_socket_addr(&peer) {
        return Ok(p);
    }

    // Try to resolve hostname
    if let Ok(mut iter) = peer.to_socket_addrs() {
        // Prefer ip4
        if let Some(p) = iter.find(|x| x.is_ipv4()) {
            return Ok(p);
        }
        if let Some(p) = iter.find(|x| x.is_ipv6()) {
            return Ok(p);
        }
    }

    // Nothing worked, return an error
    Err(TransportError::InvalidAddress)?
}

pub(super) fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse().map_err(|_| TransportError::InvalidAddress)?)
}

/// Connect to a socket address via a regular TcpStream
#[instrument(skip_all)]
pub(crate) async fn connect(
    hostname_port: &HostnamePort,
) -> Result<(OwnedReadHalf, OwnedWriteHalf)> {
    Ok(create_tcp_stream(hostname_port).await?.into_split())
}

/// Create a TCP stream to a given socket address
pub(crate) async fn create_tcp_stream(hostname_port: &HostnamePort) -> Result<TcpStream> {
    debug!(addr = %hostname_port, "Connecting");
    let connection = match TcpStream::connect(hostname_port.to_string()).await {
        Ok(c) => {
            debug!(addr = %hostname_port, "Connected");
            c
        }
        Err(e) => {
            debug!(addr = %hostname_port, err = %e, "Failed to connect");
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
    debug!(hostname_port = %hostname_port, "Trying to connect using TLS");

    // create a tcp stream
    let connection = create_tcp_stream(hostname_port).await?;

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

#[cfg(test)]
mod test {
    use crate::transport::common::parse_socket_addr;
    use core::fmt::Debug;
    use ockam_core::{Error, Result};
    use ockam_transport_core::TransportError;

    fn assert_transport_error<T>(result: Result<T>, error: TransportError)
    where
        T: Debug,
    {
        let invalid_address_error: Error = error.into();
        assert_eq!(result.unwrap_err().code(), invalid_address_error.code())
    }

    #[test]
    fn test_parse_socket_address() {
        let result = parse_socket_addr("hostname:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("example.com:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:port");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.1:80");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:65536");
        assert!(result.is_err());
        assert_transport_error(result, TransportError::InvalidAddress);

        let result = parse_socket_addr("127.0.0.1:0");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:80");
        assert!(result.is_ok());

        let result = parse_socket_addr("127.0.0.1:8080");
        assert!(result.is_ok());
    }
}
