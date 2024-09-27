use crate::ebpf_portal::{
    Inlet, InletConnection, InletRegistry, OckamPortalPacket, Outlet, OutletConnection,
    OutletRegistry, Port, RawSocketMessage,
};
use log::warn;
use ockam_core::{async_trait, route, LocalMessage, Processor, Result};
use ockam_node::Context;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::Packet;
use pnet::transport;
use pnet::transport::{
    tcp_packet_iter, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender,
};
use rand::random;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use tracing::info;

/// Processor responsible for receiving all data with OCKAM_TCP_PORTAL_PROTOCOL on the machine
/// and redirect it to individual portal workers.
pub struct RawSocketProcessor {
    socket_write_handle: Arc<RwLock<TransportSender>>,
    // TODO: Remove lock by moving to blocking and returning back from blocking thread
    socket_read_handle: Arc<RwLock<TransportReceiver>>,

    inlet_registry: InletRegistry,
    outlet_registry: OutletRegistry,
}

impl RawSocketProcessor {
    pub(crate) async fn create(
        ip_proto: u8,
        inlet_registry: InletRegistry,
        outlet_registry: OutletRegistry,
    ) -> Result<Self> {
        // FIXME: Use Layer3
        let (socket_write_handle, socket_read_handle) = transport::transport_channel(
            1024 * 1024,
            TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocol::new(
                ip_proto,
            ))),
        )
        .unwrap();

        let s = Self {
            socket_write_handle: Arc::new(RwLock::new(socket_write_handle)),
            socket_read_handle: Arc::new(RwLock::new(socket_read_handle)),
            inlet_registry,
            outlet_registry,
        };

        Ok(s)
    }

    async fn new_inlet_connection(
        inlet: &Inlet,
        src_ip: Ipv4Addr,
        parsed_packet: &ParsedPacket,
    ) -> Result<Option<Arc<InletConnection>>> {
        // TODO: eBPF Remove connection eventually

        let is_paused = inlet.inlet_shared_state.read().unwrap().is_paused;

        if is_paused {
            // Just drop the stream
            return Ok(None);
        }

        // TODO: Make sure the connection can't be spoofed by someone having access to that Outlet

        let connection = Arc::new(InletConnection {
            identifier: None,
            connection_identifier: random(),
            inlet_ip: parsed_packet.destination_ip,
            client_ip: src_ip,
            client_port: parsed_packet.source_port,
        });

        inlet.add_connection(connection.clone());

        Ok(Some(connection))
    }

    /// Write handle to the socket
    pub fn socket_write_handle(&self) -> Arc<RwLock<TransportSender>> {
        self.socket_write_handle.clone()
    }

    async fn handle_inlet(
        &self,
        ctx: &Context,
        inlet: Inlet,
        connection: &InletConnection,
        message: RawSocketMessage,
    ) -> Result<()> {
        let packet = OckamPortalPacket::from_raw_socket_message(
            message,
            connection.connection_identifier.clone(),
        );

        // debug!("Got packet, forwarding to the other side");
        info!("Got packet, forwarding to the other side");

        let inlet_shared_state = inlet.inlet_shared_state.read().unwrap().clone();

        if inlet_shared_state.is_paused {
            return Ok(());
        }

        ctx.forward_from_address(
            LocalMessage::new()
                .with_onward_route(inlet_shared_state.route)
                .with_return_route(route![inlet.portal_worker_address])
                .with_payload(minicbor::to_vec(packet)?),
            ctx.address(),
        )
        .await?;

        Ok(())
    }

    async fn handle_outlet(
        &self,
        ctx: &Context,
        outlet: Outlet,
        connection: &OutletConnection,
        message: RawSocketMessage,
    ) -> Result<()> {
        let packet = OckamPortalPacket::from_raw_socket_message(
            message,
            connection.connection_identifier.clone(),
        );

        // debug!("Got packet, forwarding to the other side");
        info!("Got packet, forwarding to the other side");

        ctx.forward_from_address(
            LocalMessage::new()
                .with_onward_route(connection.return_route.clone())
                .with_return_route(route![outlet.portal_worker_address])
                .with_payload(minicbor::to_vec(packet)?),
            ctx.address(),
        )
        .await?;

        Ok(())
    }

    async fn get_new_packet(
        socket_read_handle: Arc<RwLock<TransportReceiver>>,
    ) -> Result<Option<ParsedPacket>> {
        let parsed_packet = tokio::task::spawn_blocking(move || {
            let mut socket_read_handle = socket_read_handle.write().unwrap(); // FIXME
            let mut iterator = tcp_packet_iter(&mut socket_read_handle);
            // TODO: Should we check the checksum?
            let (packet, source_ip) = iterator.next().unwrap(); // FIXME

            let source_ip = match source_ip {
                IpAddr::V4(ip) => ip,
                IpAddr::V6(_) => return None,
            };

            let destination_ip = Ipv4Addr::LOCALHOST; // FIXME
            let source_port = packet.get_source();
            let destination_port = packet.get_destination();
            let flags = packet.get_flags();

            info!(
                "PACKET LEN: {}. Source: {}, Destination: {}",
                packet.payload().len(),
                source_port,
                destination_port,
            );

            let message = RawSocketMessage::from_packet(packet, source_ip);

            let parsed_packet = ParsedPacket {
                message,
                source_ip,
                source_port,
                flags,
                destination_ip,
                destination_port,
            };

            Some(parsed_packet)
        })
        .await
        .unwrap();

        Ok(parsed_packet)
    }
}

struct ParsedPacket {
    message: RawSocketMessage,

    source_ip: Ipv4Addr,
    source_port: Port,
    flags: u8,

    destination_ip: Ipv4Addr,
    destination_port: Port,
}

#[async_trait]
impl Processor for RawSocketProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let parsed_packet = Self::get_new_packet(self.socket_read_handle.clone()).await?;

        let parsed_packet = match parsed_packet {
            Some(parsed_packet) => parsed_packet,
            None => return Ok(false),
        };

        if let Some(inlet) = self
            .inlet_registry
            .get_inlet(parsed_packet.destination_port)
        {
            let connection = match inlet
                .get_connection_internal(parsed_packet.source_ip, parsed_packet.source_port)
            {
                Some(connection) => {
                    // trace!("Existing connection from {}", packet.get_source());
                    info!("Existing connection from {}", parsed_packet.source_port);
                    connection
                }
                None => {
                    if parsed_packet.flags != 2 {
                        warn!(
                            "Unknown connection packet from {}. Skipping",
                            parsed_packet.source_port
                        );
                        return Ok(true);
                    }

                    // debug!("New connection from {}", packet.get_source());
                    info!("New connection from {}", parsed_packet.source_port);
                    match Self::new_inlet_connection(
                        &inlet,
                        parsed_packet.source_ip,
                        &parsed_packet,
                    )
                    .await?
                    {
                        Some(connection) => connection,
                        None => return Ok(true),
                    }
                }
            };

            self.handle_inlet(ctx, inlet, &connection, parsed_packet.message)
                .await?;

            return Ok(true);
        }

        let outlet = match self
            .outlet_registry
            .get_outlet(parsed_packet.source_ip, parsed_packet.source_port)
        {
            Some(outlet) => outlet,
            None => return Ok(true),
        };

        let connection = match outlet.get_connection_internal(parsed_packet.destination_port) {
            Some(connection) => {
                // trace!("Existing connection to {}", packet.get_destination());
                info!("Existing connection to {}", parsed_packet.destination_port);
                connection
            }
            None => return Ok(true),
        };

        self.handle_outlet(ctx, outlet, &connection, parsed_packet.message)
            .await?;

        Ok(true)
    }
}
