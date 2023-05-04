use core::fmt;
use core::fmt::Formatter;
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Result};
use ockam_transport_core::TransportError;

/// Result of [`TcpTransport::connect`] call.
#[derive(Clone, Debug)]
pub struct TcpConnection {
    sender_address: Address,
    socket_address: SocketAddr,
    flow_control_id: FlowControlId,
}

impl fmt::Display for TcpConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Socket: {}, Worker: {}, FlowId: {}",
            self.socket_address, self.sender_address, self.flow_control_id
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
        socket_address: SocketAddr,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            sender_address,
            socket_address,
            flow_control_id,
        }
    }
    /// Corresponding [`TcpSendWorker`](super::workers::TcpSendWorker) [`Address`] that can be used in a route to send messages to the other
    /// side of the TCP connection
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }
    /// Corresponding [`SocketAddr`]
    pub fn socket_address(&self) -> &SocketAddr {
        &self.socket_address
    }
    /// Generated fresh random [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
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
pub(super) fn resolve_peer(peer: String) -> Result<SocketAddr> {
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
    Err(TransportError::InvalidAddress.into())
}

pub(super) fn parse_socket_addr(s: &str) -> Result<SocketAddr> {
    Ok(s.parse().map_err(|_| TransportError::InvalidAddress)?)
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
