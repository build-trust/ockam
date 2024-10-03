use crate::ebpf_portal::{
    Inlet, InletRegistry, Outlet, OutletRegistry, ParsedRawSocketPacket, RawSocketPacket,
};
use ockam_core::{async_trait, Processor, Result};
use ockam_node::Context;
use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::Packet;
use pnet::transport;
use pnet::transport::{
    tcp_packet_iter, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender,
};
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

    /// Write handle to the socket
    pub fn socket_write_handle(&self) -> Arc<RwLock<TransportSender>> {
        self.socket_write_handle.clone()
    }

    async fn handle_inlet(&self, inlet: Inlet, packet: ParsedRawSocketPacket) -> Result<()> {
        inlet.sender.send(packet).await.unwrap(); //FIXME

        Ok(())
    }

    async fn handle_outlet(&self, outlet: Outlet, packet: ParsedRawSocketPacket) -> Result<()> {
        outlet.sender.send(packet).await.unwrap();

        Ok(())
    }

    async fn get_new_packet(
        socket_read_handle: Arc<RwLock<TransportReceiver>>,
    ) -> Result<Option<ParsedRawSocketPacket>> {
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

            info!(
                "PACKET LEN: {}. Source: {}, Destination: {}",
                packet.payload().len(),
                source_port,
                destination_port,
            );

            let packet = RawSocketPacket::from_packet(packet, source_ip);

            let parsed_packet = ParsedRawSocketPacket {
                packet,
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

#[async_trait]
impl Processor for RawSocketProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        let parsed_packet = Self::get_new_packet(self.socket_read_handle.clone()).await?;

        let parsed_packet = match parsed_packet {
            Some(parsed_packet) => parsed_packet,
            None => return Ok(false),
        };

        if let Some(inlet) = self
            .inlet_registry
            .get_inlet(parsed_packet.destination_port)
        {
            self.handle_inlet(inlet, parsed_packet).await?;

            return Ok(true);
        }

        if let Some(outlet) = self
            .outlet_registry
            .get_outlet(parsed_packet.packet.source_ip, parsed_packet.packet.source)
        {
            self.handle_outlet(outlet, parsed_packet).await?;

            return Ok(true);
        };

        Ok(true)
    }
}
