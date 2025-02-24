use crate::portal::InletRouteMutableState;
use crate::privileged_portal::packet::RawSocketReadResult;
use crate::privileged_portal::{Inlet, InletConnection, OckamPortalPacket, Outlet, PortalMode};
use log::{debug, trace, warn};
use ockam_core::{async_trait, cbor_encode_preallocate, route, LocalMessage, Processor, Result};
use ockam_node::Context;
use rand::random;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

/// Processor handles all packets for the corresponding Inlet or Outlet.
/// Packets are read by [`RawSocketProcessor`] and redirected here.
pub struct InternalProcessor {
    mode: PortalMode,

    receiver: Receiver<RawSocketReadResult>,
}

impl InternalProcessor {
    /// Constructor.
    pub fn new_inlet(receiver: Receiver<RawSocketReadResult>, inlet: Inlet) -> Self {
        Self {
            mode: PortalMode::Inlet { inlet },
            receiver,
        }
    }

    /// Constructor.
    pub fn new_outlet(receiver: Receiver<RawSocketReadResult>, outlet: Outlet) -> Self {
        Self {
            mode: PortalMode::Outlet { outlet },
            receiver,
        }
    }

    async fn new_inlet_connection(
        inlet: &Inlet,
        inlet_mutable_route_state: InletRouteMutableState,
        src_ip: Ipv4Addr,
        raw_socket_read_result: &RawSocketReadResult,
    ) -> Result<Arc<InletConnection>> {
        // TODO: eBPF Remove connection eventually

        let connection = Arc::new(InletConnection {
            inlet_mutable_route_state,
            connection_identifier: random(),
            inlet_ip: raw_socket_read_result.ipv4_info.destination_ip(),
            client_ip: src_ip,
            client_port: raw_socket_read_result.tcp_info.source_port(),
        });

        inlet.add_connection(connection.clone());

        Ok(connection)
    }
}

#[async_trait]
impl Processor for InternalProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let raw_socket_read_result = match self.receiver.recv().await {
            Some(raw_socket_read_result) => raw_socket_read_result,
            None => return Ok(false),
        };

        match &self.mode {
            // Client -> Inlet packet
            PortalMode::Inlet { inlet } => {
                let connection = match inlet.get_connection_internal(
                    raw_socket_read_result.ipv4_info.source_ip(),
                    raw_socket_read_result.tcp_info.source_port(),
                ) {
                    Some(connection) => {
                        trace!(
                            "Inlet Processor: Existing connection from {}:{}",
                            raw_socket_read_result.ipv4_info.source_ip(),
                            raw_socket_read_result.tcp_info.source_port(),
                        );
                        connection
                    }
                    None => {
                        if !raw_socket_read_result.tcp_info.is_syn_only() {
                            warn!(
                                "Inlet Processor: Unknown connection packet from {}:{}. Skipping",
                                raw_socket_read_result.ipv4_info.source_ip(),
                                raw_socket_read_result.tcp_info.source_port(),
                            );
                            return Ok(true);
                        }

                        debug!(
                            "Inlet Processor: New connection from {}:{}",
                            raw_socket_read_result.ipv4_info.source_ip(),
                            raw_socket_read_result.tcp_info.source_port(),
                        );
                        let active_route = inlet.inlet_shared_state.choose_active_route().await;

                        Self::new_inlet_connection(
                            inlet,
                            active_route,
                            raw_socket_read_result.ipv4_info.source_ip(),
                            &raw_socket_read_result,
                        )
                        .await?
                    }
                };

                let (route_index, route) = connection.inlet_mutable_route_state.snapshot_route();
                let portal_packet = OckamPortalPacket::from_tcp_packet(
                    connection.connection_identifier.clone(),
                    route_index,
                    raw_socket_read_result.header_and_payload,
                );

                trace!("Inlet Processor: Got packet, forwarding to the other side");

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(route)
                        .with_return_route(route![inlet.remote_worker_address.clone()])
                        .with_payload(cbor_encode_preallocate(&portal_packet)?),
                    ctx.primary_address().clone(),
                )
                .await?;
            }
            // Server -> Outlet packet
            PortalMode::Outlet { outlet } => {
                let connection = match outlet
                    .get_connection_internal(raw_socket_read_result.tcp_info.destination_port())
                {
                    Some(connection) => {
                        trace!(
                            "Outlet Processor: Existing connection to {}",
                            raw_socket_read_result.tcp_info.destination_port(),
                        );
                        connection
                    }
                    None => {
                        warn!(
                            "Outlet Processor: Unknown connection packet from {}:{}. Skipping",
                            raw_socket_read_result.ipv4_info.source_ip(),
                            raw_socket_read_result.tcp_info.source_port(),
                        );
                        return Ok(true);
                    }
                };

                let portal_packet = OckamPortalPacket::from_tcp_packet(
                    connection.connection_identifier.clone(),
                    0, // Doesn't matter for the outlet, as outlets can't update the route
                    raw_socket_read_result.header_and_payload,
                );

                trace!("Outlet Processor: Got packet, forwarding to the other side");

                let return_route = connection.return_route.read().unwrap().route.clone();

                ctx.forward_from_address(
                    LocalMessage::new()
                        .with_onward_route(return_route)
                        .with_return_route(route![outlet.remote_worker_address.clone()])
                        .with_payload(cbor_encode_preallocate(&portal_packet)?),
                    ctx.primary_address().clone(),
                )
                .await?;
            }
        }

        Ok(true)
    }
}
