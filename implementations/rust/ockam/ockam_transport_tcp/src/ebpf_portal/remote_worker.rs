use crate::ebpf_portal::{
    Inlet, InletConnection, OckamPortalPacket, Outlet, OutletConnection, Port,
    TcpTransportEbpfSupport,
};
use ockam_core::{async_trait, Any, Result, Route, Routed, Worker};
use ockam_node::Context;
use pnet::packet::tcp::MutableTcpPacket;
use pnet::transport::TransportSender;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::{info, warn};

/// PortalWorker mode of operation
pub enum PortalWorkerMode {
    /// PortalWorker spawned for an Inlet
    Inlet {
        /// Inlet info
        inlet: Inlet,
    },
    /// PortalWorker spawned for an Outlet
    Outlet {
        /// Outlet info
        outlet: Outlet,
    },
}

/// Worker listens for new incoming connections.
pub struct PortalWorker {
    mode: PortalWorkerMode,

    socket_write_handle: Arc<RwLock<TransportSender>>,
    ebpf_support: TcpTransportEbpfSupport,
}

impl PortalWorker {
    /// Constructor.
    pub fn new_inlet(
        socket_write_handle: Arc<RwLock<TransportSender>>,
        inlet: Inlet,
        ebpf_support: TcpTransportEbpfSupport,
    ) -> Self {
        Self {
            mode: PortalWorkerMode::Inlet { inlet },
            socket_write_handle,
            ebpf_support,
        }
    }

    /// Constructor.
    pub fn new_outlet(
        socket_write_handle: Arc<RwLock<TransportSender>>,
        outlet: Outlet,
        ebpf_support: TcpTransportEbpfSupport,
    ) -> Self {
        Self {
            mode: PortalWorkerMode::Outlet { outlet },
            socket_write_handle,
            ebpf_support,
        }
    }

    async fn new_outlet_connection(
        &self,
        outlet: &Outlet,
        identifier: Option<String>,
        msg: &OckamPortalPacket,
        return_route: Route,
    ) -> Result<Arc<OutletConnection>> {
        // debug!("New TCP connection");
        info!("New TCP connection");

        // FIXME: eBPF It should an IP address of the network device that we'll use to send packets,
        //         However, we don't know it here.
        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let assigned_port = tcp_listener.local_addr().unwrap().port();

        let connection = Arc::new(OutletConnection {
            identifier,
            connection_identifier: msg.connection_identifier.clone(),
            assigned_port,
            _tcp_listener: Arc::new(tcp_listener),
            return_route,
        });

        outlet.add_connection(connection.clone());

        self.ebpf_support.add_outlet_port(assigned_port)?;

        Ok(connection)
    }

    async fn handle(
        &self,
        msg: OckamPortalPacket,
        src_port: Port,
        dst_ip: Ipv4Addr,
        dst_port: Port,
    ) -> Result<()> {
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

        packet.set_source(src_port);
        packet.set_destination(dst_port);

        let check = pnet::packet::tcp::ipv4_checksum(
            &packet.to_immutable(),
            // checksum is adjusted inside the eBPF in respect to the correct src IP addr
            &Ipv4Addr::new(0, 0, 0, 0),
            &dst_ip,
        );

        packet.set_checksum(check);

        let packet = packet.to_immutable();

        // TODO: We don't pick the source IP here, but it's important that it stays the same,
        //  Otherwise the receiving TCP connection would be disrupted.
        self.socket_write_handle
            .write()
            .unwrap()
            .send_to(packet, IpAddr::V4(dst_ip))
            .unwrap();

        Ok(())
    }

    async fn handle_inlet(
        &self,
        inlet: &Inlet,
        connection: &InletConnection,
        msg: OckamPortalPacket,
    ) -> Result<()> {
        self.handle(
            msg,
            inlet.port,
            connection.client_ip,
            connection.client_port,
        )
        .await
    }

    async fn handle_outlet(
        &self,
        outlet: &Outlet,
        connection: &OutletConnection,
        msg: OckamPortalPacket,
    ) -> Result<()> {
        self.handle(
            msg,
            connection.assigned_port,
            outlet.dst_ip,
            outlet.dst_port,
        )
        .await
    }
}

#[async_trait]
impl Worker for PortalWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let payload = msg.into_payload();

        let msg: OckamPortalPacket = minicbor::decode(&payload)?;

        let identifier = None; // FIXME: Should be the Identifier of the other side

        match &self.mode {
            PortalWorkerMode::Inlet { inlet } => {
                if let Some(connection) =
                    inlet.get_connection_external(identifier, msg.connection_identifier.clone())
                {
                    self.handle_inlet(inlet, &connection, msg).await?;

                    return Ok(());
                }

                warn!("Portal Worker in Inlet mode received a packet for an unknown connection");
            }
            PortalWorkerMode::Outlet { outlet } => {
                if let Some(connection) = outlet
                    .get_connection_external(identifier.clone(), msg.connection_identifier.clone())
                {
                    self.handle_outlet(outlet, &connection, msg).await?;

                    return Ok(());
                }

                if msg.flags == 2 {
                    let connection = self
                        .new_outlet_connection(outlet, identifier, &msg, return_route)
                        .await?;

                    self.handle_outlet(outlet, &connection, msg).await?;

                    return Ok(());
                }

                warn!("Portal Worker in Outlet mode received a non SYN packet for an unknown connection");
            }
        }

        Ok(())
    }
}
