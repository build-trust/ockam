use crate::ebpf_portal::{
    OckamPortalPacket, OutletMappingValue, OutletRegistry, PortalProcessor, PortalWorker,
    TcpTransportEbpfSupport,
};
use crate::portal::addresses::{Addresses, PortalType};
use crate::TcpOutletOptions;
use ockam_core::{async_trait, Address, AllowAll, Any, DenyAll, Result, Route, Routed, Worker};
use ockam_node::Context;
use pnet::transport::TransportSender;
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

/// Worker listens for new incoming connections.
pub struct OutletListenerWorker {
    options: TcpOutletOptions,

    socket_write_handle: Arc<RwLock<TransportSender>>,
    outlet_registry: OutletRegistry,

    dst_ip: Ipv4Addr,
    dst_port: u16,

    ebpf_support: TcpTransportEbpfSupport,
}

impl OutletListenerWorker {
    /// Constructor.
    pub fn new(
        options: TcpOutletOptions,
        socket_write_handle: Arc<RwLock<TransportSender>>,
        outlet_registry: OutletRegistry,
        dst_ip: Ipv4Addr,
        dst_port: u16,
        ebpf_support: TcpTransportEbpfSupport,
    ) -> Self {
        Self {
            options,
            socket_write_handle,
            outlet_registry,
            dst_ip,
            dst_port,
            ebpf_support,
        }
    }

    async fn new_outlet_connection(
        &self,
        ctx: &Context,
        src_addr: Address,
        msg: OckamPortalPacket<'_>,
        return_route: Route,
    ) -> Result<()> {
        // TODO: Remove connection eventually?

        // debug!("New TCP connection");
        info!("New TCP connection");

        let addresses = Addresses::generate(PortalType::EbpfOutlet);

        self.options
            .setup_flow_control_for_outlet(ctx.flow_controls(), &addresses, &src_addr);

        let (sender, receiver) = tokio::sync::mpsc::channel(128);

        // FIXME: eBPF Address?
        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let assigned_port = tcp_listener.local_addr().unwrap().port();

        let mapping = OutletMappingValue {
            inlet_worker_address: return_route.recipient()?,
            _addresses: addresses.clone(),
            sender,
            assigned_port,
        };

        let processor = PortalProcessor::new_outlet(
            receiver,
            addresses.clone(),
            return_route,
            tcp_listener,
            assigned_port,
            self.ebpf_support.clone(),
        );
        let worker = PortalWorker::new(
            None,
            self.socket_write_handle.clone(),
            assigned_port,
            self.dst_ip,
            self.dst_port,
            Some(msg.into_owned()),
        );

        ctx.start_processor_with_access_control(
            addresses.receiver_remote,
            processor,
            DenyAll,
            AllowAll,
        )
        .await?;
        ctx.start_worker_with_access_control(
            addresses.sender_remote,
            worker,
            AllowAll, // FIXME eBPF
            DenyAll,
        )
        .await?;

        self.outlet_registry.add_mapping(mapping.clone());

        Ok(())
    }
}

#[async_trait]
impl Worker for OutletListenerWorker {
    type Message = Any;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let src_addr = msg.src_addr();
        let inlet_worker_address = return_route.recipient()?;
        let payload = msg.into_payload();

        let msg: OckamPortalPacket = minicbor::decode(&payload)?;

        if msg.flags != 2 {
            warn!("Outlet Listener Worker received a non SYN packet");
            return Ok(());
        }

        if self
            .outlet_registry
            .get_mapping2(&inlet_worker_address)
            .is_some()
        {
            // FIXME: eBPF Should still send it
            debug!("Received another SYN for an already created connection");
            return Ok(());
        }

        self.new_outlet_connection(ctx, src_addr, msg, return_route)
            .await?;

        Ok(())
    }
}
