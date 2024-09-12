use crate::ebpf_portal::{
    InletMappingValue, InletRegistry, OutletRegistry, PortalProcessor, PortalWorker,
    RawSocketMessage,
};
use crate::portal::addresses::{Addresses, PortalType};
use crate::portal::InletSharedState;
use crate::TcpInletOptions;
use ockam_core::{async_trait, AllowAll, DenyAll, Processor, Result};
use ockam_node::Context;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::Packet;
use pnet::transport;
use pnet::transport::{
    tcp_packet_iter, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender,
};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

/// Processor responsible for receiving all data with OCKAM_TCP_PORTAL_PROTOCOL on the machine
/// and redirect it to individual portal workers.
pub struct RawSocketProcessor {
    socket_write_handle: Arc<RwLock<TransportSender>>,
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
        ctx: &Context,
        options: TcpInletOptions,
        inlet_shared_state: Arc<RwLock<InletSharedState>>,
        socket_write_handle: Arc<RwLock<TransportSender>>,
        registry: &InletRegistry,
        src_ip: Ipv4Addr,
        parsed_packet: &ParsedPacket,
    ) -> Result<Option<InletMappingValue>> {
        // TODO: eBPF Remove connection eventually

        let addresses = Addresses::generate(PortalType::EbpfInlet);

        let inlet_shared_state = inlet_shared_state.read().unwrap().clone();

        if inlet_shared_state.is_paused {
            // Just drop the stream
            return Ok(None);
        }

        options.setup_flow_control(
            ctx.flow_controls(),
            &addresses,
            inlet_shared_state.route.next()?,
        );

        // TODO: Make sure the connection can't be spoofed by someone having access to that Outlet

        let (sender, receiver) = tokio::sync::mpsc::channel(128);

        let mapping = InletMappingValue {
            client_ip: src_ip,
            client_port: parsed_packet.source,
            _addresses: addresses.clone(),
            sender,
        };

        let outlet_route = Arc::new(RwLock::new(inlet_shared_state.route));
        let processor =
            PortalProcessor::new_inlet(receiver, addresses.clone(), outlet_route.clone());
        let worker = PortalWorker::new(
            Some(outlet_route),
            socket_write_handle,
            parsed_packet.destination,
            src_ip,
            parsed_packet.source,
            None,
        );

        ctx.start_processor_with_access_control(
            addresses.receiver_remote,
            processor,
            DenyAll,
            AllowAll, // FIXME eBPF
        )
        .await?;
        ctx.start_worker_with_access_control(
            addresses.sender_remote,
            worker,
            AllowAll, // FIXME eBPF
            DenyAll,
        )
        .await?;

        registry.add_mapping(mapping.clone());

        Ok(Some(mapping))
    }

    /// Write handle to the socket
    pub fn socket_write_handle(&self) -> Arc<RwLock<TransportSender>> {
        self.socket_write_handle.clone()
    }
}

struct ParsedPacket {
    message: RawSocketMessage,

    source_ip: Ipv4Addr,
    flags: u8,
    source: u16,
    destination: u16,
}

#[async_trait]
impl Processor for RawSocketProcessor {
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let socket_read_handle = self.socket_read_handle.clone();
        let parsed_packet = tokio::task::spawn_blocking(move || {
            let mut socket_read_handle = socket_read_handle.write().unwrap(); // FIXME
            let mut iterator = tcp_packet_iter(&mut socket_read_handle);
            // TODO: Should we check the checksum?
            let (packet, source_ip) = iterator.next().unwrap(); // FIXME

            let source_ip = match source_ip {
                IpAddr::V4(ip) => ip,
                IpAddr::V6(_) => return None,
            };

            let source = packet.get_source();
            let destination = packet.get_destination();
            let flags = packet.get_flags();

            info!(
                "PACKET LEN: {}. Source: {}, Destination: {}",
                packet.payload().len(),
                source,
                destination,
            );

            let message = RawSocketMessage::from_packet(packet, source_ip);

            let parsed_packet = ParsedPacket {
                message,
                source_ip,
                flags,
                source,
                destination,
            };

            Some(parsed_packet)
        })
        .await
        .unwrap();

        let parsed_packet = match parsed_packet {
            Some(parsed_packet) => parsed_packet,
            None => return Ok(false),
        };

        if let Some((inlet_shared_state, options)) = self
            .inlet_registry
            .get_inlets_info(parsed_packet.destination)
        {
            let mapping = match self
                .inlet_registry
                .get_mapping(parsed_packet.source_ip, parsed_packet.source)
            {
                Some(mapping) => {
                    // trace!("Existing connection from {}", packet.get_source());
                    info!("Existing connection from {}", parsed_packet.source);
                    mapping
                }
                None => {
                    if parsed_packet.flags != 2 {
                        warn!(
                            "Unknown connection packet from {}. Skipping",
                            parsed_packet.source
                        );
                        return Ok(true);
                    }

                    // debug!("New connection from {}", packet.get_source());
                    info!("New connection from {}", parsed_packet.source);
                    match Self::new_inlet_connection(
                        ctx,
                        options,
                        inlet_shared_state,
                        self.socket_write_handle.clone(),
                        &self.inlet_registry,
                        parsed_packet.source_ip,
                        &parsed_packet,
                    )
                    .await?
                    {
                        Some(mapping) => mapping,
                        None => return Ok(true),
                    }
                }
            };

            mapping.sender.send(parsed_packet.message).await.unwrap();

            return Ok(true);
        }

        let _outlet = match self
            .outlet_registry
            .get_outlet(parsed_packet.source_ip, parsed_packet.source)
        {
            Some(outlet) => outlet,
            None => return Ok(true),
        };

        let mapping = match self.outlet_registry.get_mapping(parsed_packet.destination) {
            Some(mapping) => {
                // trace!("Existing connection to {}", packet.get_destination());
                info!("Existing connection to {}", parsed_packet.destination);
                mapping
            }
            None => return Ok(true),
        };

        mapping.sender.send(parsed_packet.message).await.unwrap();

        Ok(true)
    }
}
