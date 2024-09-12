use crate::ebpf_portal::OckamPortalPacket;
use ockam_core::{async_trait, Any, Result, Route, Routed, Worker};
use ockam_node::Context;
use pnet::packet::tcp::MutableTcpPacket;
use pnet::transport::TransportSender;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use tracing::info;

/// Worker responsible for writing data to the Socket that is received from the other side of the
/// TCP connection.
pub struct PortalWorker {
    current_route: Option<Arc<RwLock<Route>>>,
    // FIXME: eBPF I doubt there should be a mutable usage
    socket_write_handle: Arc<RwLock<TransportSender>>,
    src_port: u16,
    dst_ip: Ipv4Addr,
    dst_port: u16,

    first_message: Option<OckamPortalPacket<'static>>,
}

impl PortalWorker {
    /// Constructor.
    pub fn new(
        current_route: Option<Arc<RwLock<Route>>>,
        socket_write_handle: Arc<RwLock<TransportSender>>,
        src_port: u16,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        first_message: Option<OckamPortalPacket<'static>>,
    ) -> Self {
        Self {
            current_route,
            socket_write_handle,
            src_port,
            dst_ip,
            dst_port,
            first_message,
        }
    }

    async fn handle(&self, msg: OckamPortalPacket<'_>) -> Result<()> {
        let buff_len = (msg.data_offset as usize) * 4 + msg.payload.len();

        let buff = vec![0u8; buff_len];
        let mut packet = MutableTcpPacket::owned(buff).unwrap();

        packet.set_sequence(msg.sequence);
        packet.set_acknowledgement(msg.acknowledgement);
        packet.set_data_offset(msg.data_offset);
        packet.set_reserved(msg.reserved);
        packet.set_flags(msg.flags);
        packet.set_window(msg.window);
        packet.set_urgent_ptr(msg.urgent_ptr);
        packet.set_options(
            msg.options
                .into_iter()
                .map(Into::into)
                .collect::<Vec<pnet::packet::tcp::TcpOption>>()
                .as_slice(),
        );
        packet.set_payload(&msg.payload);

        packet.set_source(self.src_port);
        packet.set_destination(self.dst_port);

        let check = pnet::packet::tcp::ipv4_checksum(
            &packet.to_immutable(),
            // checksum is adjusted inside the eBPF in respect to the correct src IP addr
            &Ipv4Addr::new(0, 0, 0, 0),
            &self.dst_ip,
        );

        packet.set_checksum(check);

        let packet = packet.to_immutable();

        self.socket_write_handle
            .write()
            .unwrap()
            .send_to(packet, IpAddr::V4(self.dst_ip))
            .unwrap();

        Ok(())
    }
}

#[async_trait]
impl Worker for PortalWorker {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        if let Some(msg) = self.first_message.take() {
            self.handle(msg).await?;
        }

        Ok(())
    }

    async fn handle_message(
        &mut self,
        _ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // debug!("Got message, forwarding to the socket");
        info!("Got message, forwarding to the socket");

        if let Some(current_route) = &self.current_route {
            *current_route.write().unwrap() = msg.return_route();
        }

        let payload = msg.into_payload();

        let msg: OckamPortalPacket = minicbor::decode(&payload)?;

        self.handle(msg).await
    }
}
