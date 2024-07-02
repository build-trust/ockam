use crate::workers::{Addresses, TransportMessageCodec, UdpReceiverProcessor, UdpSenderWorker};
use crate::{UdpBindOptions, UdpTransport};
use core::fmt;
use core::fmt::Formatter;
use futures_util::StreamExt;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, AllowAll, DenyAll, Error, Result};
use ockam_node::{ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::{parse_socket_addr, resolve_peer, TransportError};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;
use tracing::{debug, error};

/// UDP bind arguments
pub struct UdpBindArguments {
    /// Whether we communicate with one specific peer
    peer_address: Option<SocketAddr>,
    /// Local bind address
    bind_address: SocketAddr,
}

impl Default for UdpBindArguments {
    fn default() -> Self {
        Self {
            peer_address: None,
            bind_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
        }
    }
}

impl UdpBindArguments {
    /// Default arguments with bind address 127.0.0.1:0
    pub fn new() -> Self {
        Self::default()
    }

    /// Set local bind address
    pub fn with_bind_address(mut self, bind_address: impl AsRef<str>) -> Result<Self> {
        let bind_address = parse_socket_addr(bind_address.as_ref())?;
        self.bind_address = bind_address;

        Ok(self)
    }

    /// Set peer address if we communicate with one specific peer
    pub fn with_peer_address(mut self, peer_address: impl AsRef<str>) -> Result<Self> {
        let peer_address = resolve_peer(peer_address.as_ref().to_string())?;
        self.peer_address = Some(peer_address);

        Ok(self)
    }
}

impl UdpTransport {
    /// Bind to a local port
    pub async fn bind(
        &self,
        arguments: UdpBindArguments,
        options: UdpBindOptions,
    ) -> Result<UdpBind> {
        // This transport only supports IPv4
        if !arguments.bind_address.is_ipv4() {
            error!(local_addr = %arguments.bind_address, "This transport only supports IPv4");
            return Err(TransportError::InvalidAddress(
                arguments.bind_address.to_string(),
            ))?;
        }

        // Bind new socket
        let socket = UdpSocket::bind(arguments.bind_address)
            .await
            .map_err(|_| TransportError::BindFailed)?;

        if let Some(_peer) = &arguments.peer_address {
            // TODO: Would be better to tie this socket to a specific peer when
            //  we know it beforehand, so that traffic from other peers is dropped before it gets
            //  to us, however this seems to not work for some reason, so the traffic is filtered
            //  manually in the Receiver Processor.
            // socket.connect(peer).await.unwrap();
        }

        let local_addr = socket
            .local_addr()
            .map_err(|_| Error::new(Origin::Transport, Kind::Io, "invalid local address"))?;

        // Split socket into sink and stream
        let (sink, stream) = UdpFramed::new(socket, TransportMessageCodec).split();

        let addresses = Addresses::generate();

        debug!("Creating UDP sender and receiver. Peer: {:?}, Local address: {}, Sender: {}, Receiver: {}",
            arguments.peer_address,
            local_addr,
            addresses.sender_address(),
            addresses.receiver_address());

        options.setup_flow_control(self.ctx.flow_controls(), &addresses);
        let flow_control_id = options.flow_control_id.clone();
        let receiver_outgoing_access_control =
            options.create_receiver_outgoing_access_control(self.ctx.flow_controls());

        let sender = UdpSenderWorker::new(addresses.clone(), sink, arguments.peer_address);
        WorkerBuilder::new(sender)
            .with_address(addresses.sender_address().clone())
            .with_incoming_access_control(AllowAll)
            .with_outgoing_access_control(DenyAll)
            .start(&self.ctx)
            .await?;

        let receiver = UdpReceiverProcessor::new(addresses.clone(), stream, arguments.peer_address);
        ProcessorBuilder::new(receiver)
            .with_address(addresses.receiver_address().clone())
            .with_incoming_access_control(DenyAll)
            .with_outgoing_access_control_arc(receiver_outgoing_access_control)
            .start(&self.ctx)
            .await?;

        let bind = UdpBind::new(
            addresses,
            arguments.peer_address,
            local_addr,
            flow_control_id,
        );

        Ok(bind)
    }

    /// Interrupt an active TCP connection given its Sender `Address`
    pub async fn unbind(&self, address: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(address.into()).await
    }
}

/// Result of [`TcpTransport::listen`] call.
#[derive(Clone, Debug)]
pub struct UdpBind {
    addresses: Addresses,
    peer: Option<SocketAddr>,
    bind_address: SocketAddr,
    flow_control_id: FlowControlId,
}

impl fmt::Display for UdpBind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Peer: {:?}, Bind: {}, Receiver: {}, Sender: {}, FlowId: {}",
            self.peer,
            self.bind_address,
            self.addresses.receiver_address(),
            self.addresses.sender_address(),
            self.flow_control_id
        )
    }
}

impl UdpBind {
    /// Constructor
    pub(crate) fn new(
        addresses: Addresses,
        peer: Option<SocketAddr>,
        bind_address: SocketAddr,
        flow_control_id: FlowControlId,
    ) -> Self {
        Self {
            addresses,
            peer,
            bind_address,
            flow_control_id,
        }
    }

    /// Receiver processor Address
    pub fn receiver_address(&self) -> &Address {
        self.addresses.receiver_address()
    }

    /// Sender worker address
    pub fn sender_address(&self) -> &Address {
        self.addresses.sender_address()
    }

    /// Peer if we communicate with one specific peer
    pub fn peer(&self) -> Option<SocketAddr> {
        self.peer
    }

    /// Local bind address
    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }

    /// Flow control id
    pub fn flow_control_id(&self) -> &FlowControlId {
        &self.flow_control_id
    }
}

impl From<UdpBind> for Address {
    fn from(value: UdpBind) -> Self {
        value.addresses.sender_address().clone()
    }
}
