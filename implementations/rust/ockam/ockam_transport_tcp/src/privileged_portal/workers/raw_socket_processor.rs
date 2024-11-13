use crate::privileged_portal::packet::RawSocketReadResult;
use crate::privileged_portal::{
    create_async_fd_raw_socket, Inlet, InletRegistry, Outlet, OutletRegistry, TcpPacketReader,
    TcpPacketWriter,
};
use log::trace;
use ockam_core::{async_trait, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;

/// Processor responsible for receiving all data with OCKAM_TCP_PORTAL_PROTOCOL on the machine
/// and redirect it to individual portal workers.
pub struct RawSocketProcessor {
    tcp_packet_reader: Box<dyn TcpPacketReader>,

    inlet_registry: InletRegistry,
    outlet_registry: OutletRegistry,
}

impl RawSocketProcessor {
    pub(crate) async fn create(
        ip_proto: u8,
        inlet_registry: InletRegistry,
        outlet_registry: OutletRegistry,
    ) -> Result<(Self, Box<dyn TcpPacketWriter>)> {
        let (tcp_packet_writer, tcp_packet_reader) = create_async_fd_raw_socket(ip_proto)?;

        let s = Self {
            tcp_packet_reader,
            inlet_registry,
            outlet_registry,
        };

        Ok((s, tcp_packet_writer))
    }

    async fn handle_inlet(&self, inlet: Inlet, packet: RawSocketReadResult) -> Result<()> {
        Ok(inlet
            .sender
            .send(packet)
            .await
            .map_err(|_| TransportError::RawSocketRedirectToInlet)?)
    }

    async fn handle_outlet(&self, outlet: Outlet, packet: RawSocketReadResult) -> Result<()> {
        Ok(outlet
            .sender
            .send(packet)
            .await
            .map_err(|_| TransportError::RawSocketRedirectToOutlet)?)
    }

    async fn read_packet(&mut self) -> Result<RawSocketReadResult> {
        // TODO: Should we check the TCP checksum?
        let raw_socket_read_result = self.tcp_packet_reader.read_packet().await?;

        let source_ip = raw_socket_read_result.ipv4_info.source_ip();
        let destination_ip = raw_socket_read_result.ipv4_info.destination_ip();

        let source_port = raw_socket_read_result.tcp_info.source_port();
        let destination_port = raw_socket_read_result.tcp_info.destination_port();

        trace!(
            "RAW SOCKET RECEIVED PACKET. Len: {}. Source: {}:{}, Destination: {}:{}. SYN {} ACK {} FIN {} RST {}",
            raw_socket_read_result.header_and_payload.len(),
            source_ip, source_port,
            destination_ip, destination_port,
            raw_socket_read_result.tcp_info.syn() as u8,
            raw_socket_read_result.tcp_info.ack() as u8,
            raw_socket_read_result.tcp_info.fin() as u8,
            raw_socket_read_result.tcp_info.rst() as u8
        );

        Ok(raw_socket_read_result)
    }
}

#[async_trait]
impl Processor for RawSocketProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        let raw_socket_read_result = self.read_packet().await?;

        if let Some(inlet) = self
            .inlet_registry
            .get_inlet(raw_socket_read_result.tcp_info.destination_port())
        {
            trace!(
                "Redirecting RawSocket packet to the Inlet. {}:{} -> {}:{}",
                raw_socket_read_result.ipv4_info.source_ip(),
                raw_socket_read_result.tcp_info.source_port(),
                raw_socket_read_result.ipv4_info.destination_ip(),
                raw_socket_read_result.tcp_info.destination_port()
            );
            self.handle_inlet(inlet, raw_socket_read_result).await?;

            return Ok(true);
        }

        if let Some(outlet) = self.outlet_registry.get_outlet(
            raw_socket_read_result.ipv4_info.source_ip(),
            raw_socket_read_result.tcp_info.source_port(),
        ) {
            trace!(
                "Redirecting RawSocket packet to the Outlet. {}:{} -> {}:{}",
                raw_socket_read_result.ipv4_info.source_ip(),
                raw_socket_read_result.tcp_info.source_port(),
                raw_socket_read_result.ipv4_info.destination_ip(),
                raw_socket_read_result.tcp_info.destination_port()
            );
            self.handle_outlet(outlet, raw_socket_read_result).await?;

            return Ok(true);
        };

        trace!(
            "RawSocket skipping packet. {}:{} -> {}:{}",
            raw_socket_read_result.ipv4_info.source_ip(),
            raw_socket_read_result.tcp_info.source_port(),
            raw_socket_read_result.ipv4_info.destination_ip(),
            raw_socket_read_result.tcp_info.destination_port()
        );

        Ok(true)
    }
}
