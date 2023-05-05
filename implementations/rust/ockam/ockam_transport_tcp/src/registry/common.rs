use core::fmt;
use core::fmt::Formatter;
use ockam_core::flow_control::FlowControlId;
use ockam_core::Address;
use std::net::SocketAddr;

/// Tcp connection mode
#[derive(Copy, Debug, Clone)]
pub enum TcpConnectionMode {
    /// Connection was initiated from our node
    Outgoing,
    /// Connection was accepted from a TCP listener
    Incoming,
}

impl fmt::Display for TcpConnectionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TcpConnectionMode::Outgoing => write!(f, "outgoing"),
            TcpConnectionMode::Incoming => write!(f, "responder"),
        }
    }
}

/// Information about specific Tcp sender (corresponds to one specific Tcp connection)
#[derive(Debug, Clone)]
pub struct TcpSenderInfo {
    address: Address,
    receiver_address: Address,
    socket_address: SocketAddr,
    mode: TcpConnectionMode,
    flow_control_id: FlowControlId,
}

impl TcpSenderInfo {
    /// Constructor
    pub fn new(
        address: Address,
        receiver_address: Address,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            address,
            receiver_address,
            socket_address,
            mode,
            flow_control_id,
        }
    }

    /// Address of the Sender worker
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Corresponding Tcp Receiver Processor Address
    pub fn receiver_address(&self) -> &Address {
        &self.receiver_address
    }
    /// Corresponding socket address
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address
    }
    /// Corresponding [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// [`TcpConnectionMode`] for this connection
    pub fn mode(&self) -> &TcpConnectionMode {
        &self.mode
    }
}

/// Information about specific Tcp sender (corresponds to one specific Tcp connection)
#[derive(Debug, Clone)]
pub struct TcpReceiverInfo {
    address: Address,
    sender_address: Address,
    socket_address: SocketAddr,
    mode: TcpConnectionMode,
    flow_control_id: FlowControlId,
}

impl TcpReceiverInfo {
    /// Constructor
    pub fn new(
        address: Address,
        sender_address: Address,
        socket_address: SocketAddr,
        mode: TcpConnectionMode,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            address,
            sender_address,
            socket_address,
            mode,
            flow_control_id,
        }
    }

    /// Address of the Receiver processor
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Corresponding Sender Worker Address
    pub fn sender_address(&self) -> &Address {
        &self.sender_address
    }
    /// Corresponding socket address
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address
    }
    /// Corresponding [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
    /// [`TcpConnectionMode`] for this connection
    pub fn mode(&self) -> &TcpConnectionMode {
        &self.mode
    }
}

/// Information about specific Tcp listener
#[derive(Debug, Clone)]
pub struct TcpListenerInfo {
    address: Address,
    socket_address: SocketAddr,
    flow_control_id: FlowControlId,
}

impl TcpListenerInfo {
    /// Constructor
    pub fn new(
        address: Address,
        socket_address: SocketAddr,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            address,
            socket_address,
            flow_control_id,
        }
    }

    /// Address of the Processor
    pub fn address(&self) -> &Address {
        &self.address
    }
    /// Corresponding socket address
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address
    }
    /// Corresponding [`FlowControlId`]
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}
