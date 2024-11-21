use crate::privileged_portal::packet::TcpStrippedHeaderAndPayload;
use crate::privileged_portal::{
    ConnectionIdentifier, Inlet, InletConnection, OckamPortalPacket, Outlet, OutletConnection,
    OutletConnectionReturnRoute, Port, TcpPacketWriter, TcpTransportEbpfSupport,
};
use log::{debug, trace};
use ockam_core::{
    async_trait, Any, LocalInfoIdentifier, Result, Route, Routed, SecureChannelLocalInfo, Worker,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::warn;

/// Portal mode of operation
pub enum PortalMode {
    /// Spawned for an Inlet
    Inlet {
        /// Inlet info
        inlet: Inlet,
    },
    /// Spawned for an Outlet
    Outlet {
        /// Outlet info
        outlet: Outlet,
    },
}

/// Worker receives packets for the corresponding Outlet from the other side.
pub struct RemoteWorker {
    mode: PortalMode,

    tcp_packet_writer: Box<dyn TcpPacketWriter>,
    ebpf_support: Arc<TcpTransportEbpfSupport>,
}

impl RemoteWorker {
    /// Constructor.
    pub fn new_inlet(
        tcp_packet_writer: Box<dyn TcpPacketWriter>,
        inlet: Inlet,
        ebpf_support: Arc<TcpTransportEbpfSupport>,
    ) -> Self {
        Self {
            mode: PortalMode::Inlet { inlet },
            tcp_packet_writer,
            ebpf_support,
        }
    }

    /// Constructor.
    pub fn new_outlet(
        tcp_packet_writer: Box<dyn TcpPacketWriter>,
        outlet: Outlet,
        ebpf_support: Arc<TcpTransportEbpfSupport>,
    ) -> Self {
        Self {
            mode: PortalMode::Outlet { outlet },
            tcp_packet_writer,
            ebpf_support,
        }
    }

    async fn new_outlet_connection(
        &self,
        outlet: &Outlet,
        identifier: Option<LocalInfoIdentifier>,
        connection_identifier: ConnectionIdentifier,
        return_route: Route,
    ) -> Result<Arc<OutletConnection>> {
        // FIXME: eBPF It should an IP address of the network device that we'll use to send packets,
        //         However, we don't know it here.
        let tcp_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| TransportError::BindFailed)?;
        let local_addr = tcp_listener
            .local_addr()
            .map_err(|_| TransportError::BindFailed)?;
        let assigned_port = local_addr.port();

        debug!("New TCP connection. Assigned socket {}", local_addr);

        let connection = Arc::new(OutletConnection {
            their_identifier: identifier,
            connection_identifier,
            assigned_port,
            _tcp_listener: Arc::new(tcp_listener),
            return_route: Arc::new(RwLock::new(OutletConnectionReturnRoute::new(return_route))),
        });

        outlet.add_connection(connection.clone());

        self.ebpf_support.add_outlet_port(assigned_port)?;

        Ok(connection)
    }

    async fn handle(
        tcp_packet_writer: &mut Box<dyn TcpPacketWriter>,
        header_and_payload: TcpStrippedHeaderAndPayload<'_>,
        src_port: Port,
        dst_ip: Ipv4Addr,
        dst_port: Port,
    ) -> Result<()> {
        trace!(
            "Writing packet to the RawSocket. {} -> {}:{}",
            src_port,
            dst_ip,
            dst_port
        );

        tcp_packet_writer
            .write_packet(src_port, dst_ip, dst_port, header_and_payload)
            .await?;

        Ok(())
    }

    async fn handle_inlet(
        tcp_packet_writer: &mut Box<dyn TcpPacketWriter>,
        inlet: &Inlet,
        connection: &InletConnection,
        header_and_payload: TcpStrippedHeaderAndPayload<'_>,
    ) -> Result<()> {
        Self::handle(
            tcp_packet_writer,
            header_and_payload,
            inlet.port,
            connection.client_ip,
            connection.client_port,
        )
        .await
    }

    async fn handle_outlet(
        tcp_packet_writer: &mut Box<dyn TcpPacketWriter>,
        outlet: &Outlet,
        connection: &OutletConnection,
        route_index: u32,
        header_and_payload: TcpStrippedHeaderAndPayload<'_>,
        return_route: Route,
    ) -> Result<()> {
        {
            let mut connection_return_route = connection.return_route.write().unwrap();

            if connection_return_route.route_index < route_index {
                connection_return_route.route_index = route_index;
                connection_return_route.route = return_route;
            }
        }

        Self::handle(
            tcp_packet_writer,
            header_and_payload,
            connection.assigned_port,
            outlet.dst_ip,
            outlet.dst_port,
        )
        .await
    }
}

#[async_trait]
impl Worker for RemoteWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        _ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let their_identifier = SecureChannelLocalInfo::find_info(msg.local_message())
            .map(|l| l.their_identifier())
            .ok();
        let msg = msg.into_local_message();
        let return_route = msg.return_route;
        let payload = msg.payload;

        let msg: OckamPortalPacket = minicbor::decode(&payload)
            .map_err(|e| TransportError::InvalidOckamPortalPacket(e.to_string()))?;

        let (connection_identifier, route_index, header_and_payload) = match msg.dissolve() {
            Some(res) => res,
            None => {
                return Err(TransportError::InvalidOckamPortalPacket(
                    "Invalid length".to_string(),
                ))?
            }
        };

        match &self.mode {
            // Outlet -> Inlet packet
            PortalMode::Inlet { inlet } => {
                match inlet.get_connection_external(their_identifier, connection_identifier.clone())
                {
                    Some(connection) => {
                        Self::handle_inlet(
                            &mut self.tcp_packet_writer,
                            inlet,
                            &connection,
                            header_and_payload,
                        )
                        .await?;
                    }
                    None => {
                        warn!("Portal Worker Inlet: received a packet for an unknown connection");
                    }
                }

                return Ok(());
            }
            // Inlet -> Outlet packet
            PortalMode::Outlet { outlet } => {
                if let Some(connection) = outlet.get_connection_external(
                    their_identifier.clone(),
                    connection_identifier.clone(),
                ) {
                    Self::handle_outlet(
                        &mut self.tcp_packet_writer,
                        outlet,
                        &connection,
                        route_index,
                        header_and_payload,
                        return_route,
                    )
                    .await?;

                    return Ok(());
                }

                if header_and_payload.is_syn_only() {
                    let connection = self
                        .new_outlet_connection(
                            outlet,
                            their_identifier,
                            connection_identifier,
                            return_route.clone(),
                        )
                        .await?;

                    Self::handle_outlet(
                        &mut self.tcp_packet_writer,
                        outlet,
                        &connection,
                        route_index,
                        header_and_payload,
                        return_route,
                    )
                    .await?;

                    return Ok(());
                }

                warn!("Portal Worker Outlet: received a non SYN packet for an unknown connection");
            }
        }

        Ok(())
    }
}
