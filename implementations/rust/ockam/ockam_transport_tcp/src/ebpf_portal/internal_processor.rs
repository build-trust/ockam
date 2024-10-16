use crate::ebpf_portal::{
    Inlet, InletConnection, OckamPortalPacket, Outlet, ParsedRawSocketPacket, PortalMode,
};
use log::{debug, trace, warn};
use ockam_core::{async_trait, route, LocalMessage, Processor, Result};
use ockam_node::Context;
use rand::random;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

/// Processor handles all packets for the corresponding Inlet or Outlet.
/// Packets are read by [`RawSocketProcessor`] and redirected here.
pub struct InternalProcessor {
    mode: PortalMode,

    receiver: Receiver<ParsedRawSocketPacket>,
}

impl InternalProcessor {
    /// Constructor.
    pub fn new_inlet(receiver: Receiver<ParsedRawSocketPacket>, inlet: Inlet) -> Self {
        Self {
            mode: PortalMode::Inlet { inlet },
            receiver,
        }
    }

    /// Constructor.
    pub fn new_outlet(receiver: Receiver<ParsedRawSocketPacket>, outlet: Outlet) -> Self {
        Self {
            mode: PortalMode::Outlet { outlet },
            receiver,
        }
    }

    async fn new_inlet_connection(
        inlet: &Inlet,
        src_ip: Ipv4Addr,
        parsed_packet: &ParsedRawSocketPacket,
    ) -> Result<Arc<InletConnection>> {
        // TODO: eBPF Remove connection eventually

        // TODO: Make sure the connection can't be spoofed by someone having access to that Outlet

        let connection = Arc::new(InletConnection {
            identifier: None,
            connection_identifier: random(),
            inlet_ip: parsed_packet.destination_ip,
            client_ip: src_ip,
            client_port: parsed_packet.packet.source,
        });

        inlet.add_connection(connection.clone());

        Ok(connection)
    }
}

#[async_trait]
impl Processor for InternalProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let parsed_packet = match self.receiver.recv().await {
            Some(packet) => packet,
            None => return Ok(false),
        };

        match &self.mode {
            // Client -> Inlet packet
            PortalMode::Inlet { inlet } => {
                let inlet_shared_state = inlet.inlet_shared_state.read().unwrap().clone();

                if inlet_shared_state.is_paused() {
                    return Ok(true);
                }

                let connection = match inlet.get_connection_internal(
                    parsed_packet.packet.source_ip,
                    parsed_packet.packet.source,
                ) {
                    Some(connection) => {
                        trace!(
                            "Inlet Processor: Existing connection from {}:{}",
                            parsed_packet.packet.source_ip,
                            parsed_packet.packet.source
                        );
                        connection
                    }
                    None => {
                        // Checks that SYN flag is set, and every other flag is not set.
                        const SYN: u8 = 2;
                        if parsed_packet.packet.flags != SYN {
                            warn!(
                                "Inlet Processor: Unknown connection packet from {}:{}. Skipping",
                                parsed_packet.packet.source_ip, parsed_packet.packet.source
                            );
                            return Ok(true);
                        }

                        debug!(
                            "Inlet Processor: New connection from {}:{}",
                            parsed_packet.packet.source_ip, parsed_packet.packet.source
                        );
                        Self::new_inlet_connection(
                            inlet,
                            parsed_packet.packet.source_ip,
                            &parsed_packet,
                        )
                        .await?
                    }
                };

                let portal_packet = OckamPortalPacket::from_raw_socket_packet(
                    parsed_packet.packet,
                    connection.connection_identifier.clone(),
                    inlet_shared_state.route_index(),
                );

                trace!("Inlet Processor: Got packet, forwarding to the other side");

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(inlet_shared_state.route().clone())
                        .with_return_route(route![inlet.remote_worker_address.clone()])
                        .with_payload(minicbor::to_vec(portal_packet)?),
                    ctx.address(),
                )
                .await?;
            }
            // Server -> Outlet packet
            PortalMode::Outlet { outlet } => {
                let connection =
                    match outlet.get_connection_internal(parsed_packet.packet.destination) {
                        Some(connection) => {
                            trace!(
                                "Outlet Processor: Existing connection to {}",
                                parsed_packet.packet.destination
                            );
                            connection
                        }
                        None => {
                            warn!(
                                "Outlet Processor: Unknown connection packet from {}:{}. Skipping",
                                parsed_packet.packet.source_ip, parsed_packet.packet.source
                            );
                            return Ok(true);
                        }
                    };

                let portal_packet = OckamPortalPacket::from_raw_socket_packet(
                    parsed_packet.packet,
                    connection.connection_identifier.clone(),
                    0, // Doesn't matter for the outlet, as outlet can't update the route
                );

                trace!("Outlet Processor: Got packet, forwarding to the other side");

                let return_route = connection.return_route.read().unwrap().route.clone();

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(return_route)
                        .with_return_route(route![outlet.remote_worker_address.clone()])
                        .with_payload(minicbor::to_vec(portal_packet)?),
                    ctx.address(),
                )
                .await?;
            }
        }

        Ok(true)
    }
}
