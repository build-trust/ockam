use crate::ebpf_portal::{
    Inlet, InletConnection, OckamPortalPacket, Outlet, ParsedRawSocketPacket, PortalWorkerMode,
};
use log::warn;
use ockam_core::{async_trait, route, LocalMessage, Processor, Result};
use ockam_node::Context;
use rand::random;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::info;

/// Worker listens for new incoming connections.
pub struct InternalProcessor {
    mode: PortalWorkerMode,

    receiver: Receiver<ParsedRawSocketPacket>,
}

impl InternalProcessor {
    /// Constructor.
    pub fn new_inlet(receiver: Receiver<ParsedRawSocketPacket>, inlet: Inlet) -> Self {
        Self {
            mode: PortalWorkerMode::Inlet { inlet },
            receiver,
        }
    }

    /// Constructor.
    pub fn new_outlet(receiver: Receiver<ParsedRawSocketPacket>, outlet: Outlet) -> Self {
        Self {
            mode: PortalWorkerMode::Outlet { outlet },
            receiver,
        }
    }

    async fn new_inlet_connection(
        inlet: &Inlet,
        src_ip: Ipv4Addr,
        parsed_packet: &ParsedRawSocketPacket,
    ) -> Result<Option<Arc<InletConnection>>> {
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

        Ok(Some(connection))
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
            PortalWorkerMode::Inlet { inlet } => {
                let inlet_shared_state = inlet.inlet_shared_state.read().unwrap().clone();

                if inlet_shared_state.is_paused {
                    return Ok(true);
                }

                let connection = match inlet.get_connection_internal(
                    parsed_packet.packet.source_ip,
                    parsed_packet.packet.source,
                ) {
                    Some(connection) => {
                        // trace!("Existing connection from {}", packet.get_source());
                        info!("Existing connection from {}", parsed_packet.packet.source);
                        connection
                    }
                    None => {
                        if parsed_packet.packet.flags != 2 {
                            warn!(
                                "Unknown connection packet from {}. Skipping",
                                parsed_packet.packet.source
                            );
                            return Ok(true);
                        }

                        // debug!("New connection from {}", packet.get_source());
                        info!("New connection from {}", parsed_packet.packet.source);
                        match Self::new_inlet_connection(
                            inlet,
                            parsed_packet.packet.source_ip,
                            &parsed_packet,
                        )
                        .await?
                        {
                            Some(connection) => connection,
                            None => return Ok(true),
                        }
                    }
                };

                let portal_packet = OckamPortalPacket::from_raw_socket_packet(
                    parsed_packet.packet,
                    connection.connection_identifier.clone(),
                );

                // debug!("Got packet, forwarding to the other side");
                info!("Got packet, forwarding to the other side");

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(inlet_shared_state.route)
                        .with_return_route(route![inlet.remote_worker_address.clone()])
                        .with_payload(minicbor::to_vec(portal_packet)?),
                    ctx.address(),
                )
                .await?;
            }
            PortalWorkerMode::Outlet { outlet } => {
                let connection =
                    match outlet.get_connection_internal(parsed_packet.packet.destination) {
                        Some(connection) => {
                            // trace!("Existing connection to {}", packet.get_destination());
                            info!(
                                "Existing connection to {}",
                                parsed_packet.packet.destination
                            );
                            connection
                        }
                        None => return Ok(true),
                    };

                let portal_packet = OckamPortalPacket::from_raw_socket_packet(
                    parsed_packet.packet,
                    connection.connection_identifier.clone(),
                );

                // debug!("Got packet, forwarding to the other side");
                info!("Got packet, forwarding to the other side");

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(connection.return_route.clone())
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
