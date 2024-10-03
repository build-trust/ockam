use crate::ebpf_portal::pnet_helper::{create_raw_socket, next_tcp_packet};
use crate::ebpf_portal::{
    Inlet, InletRegistry, Outlet, OutletRegistry, ParsedRawSocketPacket, RawSocketPacket,
};
use log::{trace, warn};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use pnet::packet::Packet;
use pnet::transport::{TransportReceiver, TransportSender};
use std::sync::{Arc, Mutex};

/// Processor responsible for receiving all data with OCKAM_TCP_PORTAL_PROTOCOL on the machine
/// and redirect it to individual portal workers.
pub struct RawSocketProcessor {
    socket_read_handle: Option<TransportReceiver>,

    inlet_registry: InletRegistry,
    outlet_registry: OutletRegistry,
}

impl RawSocketProcessor {
    pub(crate) async fn create(
        ip_proto: u8,
        inlet_registry: InletRegistry,
        outlet_registry: OutletRegistry,
    ) -> Result<(Self, Arc<Mutex<TransportSender>>)> {
        let (socket_write_handle, socket_read_handle) = create_raw_socket(ip_proto)?;

        let s = Self {
            socket_read_handle: Some(socket_read_handle),
            inlet_registry,
            outlet_registry,
        };

        Ok((s, Arc::new(Mutex::new(socket_write_handle))))
    }

    async fn handle_inlet(&self, inlet: Inlet, packet: ParsedRawSocketPacket) -> Result<()> {
        Ok(inlet
            .sender
            .send(packet)
            .await
            .map_err(|_| TransportError::RawSocketRedirectToInlet)?)
    }

    async fn handle_outlet(&self, outlet: Outlet, packet: ParsedRawSocketPacket) -> Result<()> {
        Ok(outlet
            .sender
            .send(packet)
            .await
            .map_err(|_| TransportError::RawSocketRedirectToOutlet)?)
    }

    async fn get_new_packet(
        mut socket_read_handle: TransportReceiver,
    ) -> Result<(TransportReceiver, ParsedRawSocketPacket)> {
        // TODO: I wonder how bad it is to use blocking here
        //  Will it be shutdown properly eventually?
        let (socket_read_handle, parsed_packet) = tokio::task::spawn_blocking(move || {
            // TODO: Should we check the TCP checksum?
            let (packet, source_ip, destination_ip) = next_tcp_packet(&mut socket_read_handle)?;

            let source_port = packet.get_source();
            let destination_port = packet.get_destination();

            let flags = packet.get_flags();

            let ack_number = packet.get_acknowledgement();
            let syn = flags & 0b0000010 != 0;
            let ack = flags & 0b0010000 != 0;
            let fin = flags & 0b0000001 != 0;
            let rst = flags & 0b0000100 != 0;

            trace!(
                "RAW SOCKET RECEIVED PACKET. LEN: {}. Source: {}, Destination: {}. ACK={}. SYN {} ACK {} FIN {} RST {}",
                packet.payload().len(),
                source_port,
                destination_port,
                ack_number,
                syn as u8, ack as u8, fin as u8, rst as u8
            );

            let packet = RawSocketPacket::from_packet(packet, source_ip);

            let parsed_packet = ParsedRawSocketPacket {
                packet,
                destination_ip,
                destination_port,
            };

            Ok::<_, Error>((socket_read_handle, parsed_packet))
        })
        .await.map_err(|e| Error::new(Origin::Core, Kind::Internal, e))??;

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
            trace!(
                "Redirecting RawSocket packet to the Inlet. {}:{} -> {}:{}",
                parsed_packet.packet.source_ip,
                parsed_packet.packet.source,
                parsed_packet.destination_ip,
                parsed_packet.destination_port
            );
            self.handle_inlet(inlet, parsed_packet).await?;

            return Ok(true);
        }

        if let Some(outlet) = self
            .outlet_registry
            .get_outlet(parsed_packet.packet.source_ip, parsed_packet.packet.source)
        {
            trace!(
                "Redirecting RawSocket packet to the Outlet. {}:{} -> {}:{}",
                parsed_packet.packet.source_ip,
                parsed_packet.packet.source,
                parsed_packet.destination_ip,
                parsed_packet.destination_port
            );
            self.handle_outlet(outlet, parsed_packet).await?;

            return Ok(true);
        };

        warn!(
            "RawSocket skipping packet. {}:{} -> {}:{}",
            parsed_packet.packet.source_ip,
            parsed_packet.packet.source,
            parsed_packet.destination_ip,
            parsed_packet.destination_port
        );

        Ok(true)
    }
}
