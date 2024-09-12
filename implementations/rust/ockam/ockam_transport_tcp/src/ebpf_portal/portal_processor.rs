use crate::ebpf_portal::{OckamPortalPacket, Port, RawSocketMessage, TcpTransportEbpfSupport};
use crate::portal::addresses::Addresses;
use ockam_core::{async_trait, route, LocalMessage, Processor, Result, Route};
use ockam_node::Context;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tokio::sync::mpsc::Receiver;
use tracing::info;

/// Processor responsible for receiving TCP packets for a certain connection.
// TODO: eBPF Can be a worker instead?
pub(crate) struct PortalProcessor {
    receiver: Receiver<RawSocketMessage>,
    addresses: Addresses,
    current_route: Arc<RwLock<Route>>,
    assigned_port_state: Option<AssignedPortState>,
}

struct AssignedPortState {
    _tcp_listener: TcpListener, // Just hold it so that port is marked as taken
    assigned_port: Port,
    ebpf_support: TcpTransportEbpfSupport,
}

impl PortalProcessor {
    /// Constructor.
    pub fn new_inlet(
        receiver: Receiver<RawSocketMessage>,
        addresses: Addresses,
        current_route: Arc<RwLock<Route>>,
    ) -> Self {
        Self {
            receiver,
            addresses,
            current_route,
            assigned_port_state: None,
        }
    }

    /// Constructor.
    pub fn new_outlet(
        receiver: Receiver<RawSocketMessage>,
        addresses: Addresses,
        current_route: Route, // Immutable
        tcp_listener: TcpListener,
        assigned_port: Port,
        ebpf_support: TcpTransportEbpfSupport,
    ) -> Self {
        Self {
            receiver,
            addresses,
            current_route: Arc::new(RwLock::new(current_route)),
            assigned_port_state: Some(AssignedPortState {
                _tcp_listener: tcp_listener,
                assigned_port,
                ebpf_support,
            }),
        }
    }
}

#[async_trait]
impl Processor for PortalProcessor {
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        if let Some(state) = &self.assigned_port_state {
            state.ebpf_support.add_outlet_port(state.assigned_port)?;
        }

        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        if let Some(state) = &self.assigned_port_state {
            state.ebpf_support.remove_outlet_port(state.assigned_port)?;
        }

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let message = match self.receiver.recv().await {
            Some(message) => message,
            None => return Ok(false),
        };

        let packet = OckamPortalPacket::from(message);

        // debug!("Got packet, forwarding to the other side");
        info!("Got packet, forwarding to the other side");

        let current_route = self.current_route.read().unwrap().clone();
        ctx.forward_from_address(
            LocalMessage::new()
                .with_onward_route(current_route)
                .with_return_route(route![self.addresses.sender_remote.clone()])
                .with_payload(minicbor::to_vec(packet)?),
            self.addresses.receiver_remote.clone(),
        )
        .await?;

        Ok(true)
    }
}
