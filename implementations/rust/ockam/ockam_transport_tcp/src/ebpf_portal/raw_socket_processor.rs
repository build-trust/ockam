use crate::ebpf_portal::pnet_helper::{create_raw_socket, next_tcp_packet};
use crate::ebpf_portal::{
    Inlet, InletRegistry, Outlet, OutletRegistry, ParsedRawSocketPacket, RawSocketPacket,
};
use ockam_core::{async_trait, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use pnet::packet::Packet;
use pnet::transport::{TransportReceiver, TransportSender};
use std::sync::{Arc, Mutex};
use tracing::info;

/// Processor responsible for receiving all data with OCKAM_TCP_PORTAL_PROTOCOL on the machine
/// and redirect it to individual portal workers.
pub struct RawSocketProcessor {
    socket_write_handle: Arc<Mutex<TransportSender>>,
    socket_read_handle: Option<TransportReceiver>,

    inlet_registry: InletRegistry,
    outlet_registry: OutletRegistry,
}

impl RawSocketProcessor {
    pub(crate) async fn create(
        ip_proto: u8,
        inlet_registry: InletRegistry,
        outlet_registry: OutletRegistry,
    ) -> Result<Self> {
        let (socket_write_handle, socket_read_handle) = create_raw_socket(ip_proto)?;

        let s = Self {
            socket_write_handle: Arc::new(Mutex::new(socket_write_handle)),
            socket_read_handle: Some(socket_read_handle),
            inlet_registry,
            outlet_registry,
        };

        Ok(s)
    }

    /// Write handle to the socket
    pub fn socket_write_handle(&self) -> Arc<Mutex<TransportSender>> {
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
        mut socket_read_handle: TransportReceiver,
    ) -> Result<(TransportReceiver, ParsedRawSocketPacket)> {
        // TODO: I wonder how bad it is to use blocking here
        //  Will it be shutdown properly eventually?
        //  Should we use socket read with timeout?
        let (socket_read_handle, parsed_packet) = tokio::task::spawn_blocking(move || {
            // TODO: Should we check the checksum?
            let (packet, source_ip, destination_ip) = next_tcp_packet(&mut socket_read_handle)
                .map_err(|_| TransportError::RawSocketReadError)?;

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

            Ok::<_, TransportError>((socket_read_handle, parsed_packet))
        })
        .await
        .unwrap()?;

        Ok((socket_read_handle, parsed_packet))
    }
}

#[async_trait]
impl Processor for RawSocketProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        // This trick allows avoiding locking around socket_read_handle
        let socket_read_handle = match self.socket_read_handle.take() {
            Some(socket_read_handle) => socket_read_handle,
            None => return Ok(false),
        };

        let (socket_read_handle, parsed_packet) = Self::get_new_packet(socket_read_handle).await?;
        self.socket_read_handle = Some(socket_read_handle);

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
