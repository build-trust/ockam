use crate::portal::InletSharedState;
use crate::privileged_portal::{InternalProcessor, Port, RemoteWorker};
use crate::{TcpInlet, TcpInletOptions, TcpOutletOptions, TcpTransport};
use caps::Capability::{CAP_BPF, CAP_NET_ADMIN, CAP_NET_RAW, CAP_SYS_ADMIN};
use caps::{CapSet, Capability};
use core::fmt::Debug;
use log::{debug, error};
use nix::unistd::Uid;
use ockam_core::{Address, DenyAll, Result, Route};
use ockam_node::compat::asynchronous::{resolve_peer, RwLock};
use ockam_node::{ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::{HostnamePort, TransportError};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::channel;
use tracing::instrument;

impl TcpTransport {
    /// Check if privileged portals can be run with current permissions
    pub fn check_capabilities() -> Result<()> {
        let caps = caps::read(None, CapSet::Effective)
            .map_err(|e| TransportError::ReadCaps(e.to_string()))?;

        const REQUIRED_SET: &[Capability] = &[CAP_NET_RAW, CAP_BPF, CAP_SYS_ADMIN, CAP_NET_ADMIN];

        let mut error_description = String::new();
        let mut check_result = true;
        for cap in REQUIRED_SET {
            if !caps.contains(cap) {
                check_result = false;
                let err = format!("{} capability is not effective", cap);
                error_description.push_str(&err);
                error_description.push_str(". ");
                error!("{}", err);
            }
        }

        if !Uid::effective().is_root() {
            error_description.push_str("User is not root");
            error!("Current user is not root. eBPF requires root.");
        }

        if !check_result {
            error!("Capabilities: {:?}", caps);
            return Err(TransportError::PrivilegedPortalsPrerequisitesCheckFailed(
                error_description,
            ))?;
        }

        debug!("Ebpf prerequisites check passed");

        Ok(())
    }

    /// Create a Privileged Inlet
    #[instrument(skip(self), fields(outlet_route=?outlet_route.clone()))]
    pub async fn create_privileged_inlet(
        &self,
        bind_addr: impl Into<String> + Clone + Debug,
        outlet_route: impl Into<Route> + Clone + Debug,
        options: TcpInletOptions,
    ) -> Result<TcpInlet> {
        Self::check_capabilities()?;

        let outlet_route = outlet_route.into();

        let next = outlet_route.next().cloned()?;

        let bind_addr = bind_addr.into();
        let tcp_listener = TcpListener::bind(bind_addr.clone())
            .await
            .map_err(|_| TransportError::BindFailed)?;
        let local_address = tcp_listener
            .local_addr()
            .map_err(|_| TransportError::BindFailed)?;

        if !local_address.ip().is_ipv4() {
            return Err(TransportError::ExpectedIPv4Address)?;
        };

        let port = local_address.port();

        // Trigger immediate attach
        self.ebpf_support.attach_ebpf_to_all_interfaces().await?;
        // Start periodic updates if needed
        self.ebpf_support
            .attach_ebpf_to_all_interfaces_start_task()
            .await;

        let tcp_packet_writer = self.start_raw_socket_processor_if_needed().await?;

        let inlet_shared_state =
            InletSharedState::create(self.ctx(), outlet_route.clone(), false).await?;
        let inlet_shared_state = Arc::new(RwLock::new(inlet_shared_state));

        let remote_worker_address = Address::random_tagged("Ebpf.RemoteWorker.Inlet");
        let internal_worker_address = Address::random_tagged("Ebpf.InternalWorker.Inlet");

        TcpInletOptions::setup_flow_control_for_address(
            self.ctx().flow_controls(),
            remote_worker_address.clone(),
            &next,
        );

        let (sender, receiver) = channel(20); // FIXME

        let inlet_info = self.ebpf_support.inlet_registry.create_inlet(
            remote_worker_address.clone(),
            internal_worker_address.clone(),
            sender,
            port,
            tcp_listener,
            inlet_shared_state.clone(),
        );

        self.ebpf_support.add_inlet_port(port)?;

        let remote_worker = RemoteWorker::new_inlet(
            tcp_packet_writer,
            inlet_info.clone(),
            self.ebpf_support.clone(),
        );
        WorkerBuilder::new(remote_worker)
            .with_address(remote_worker_address.clone())
            .with_incoming_access_control_arc(options.incoming_access_control)
            .with_outgoing_access_control(DenyAll)
            .start(self.ctx())
            .await?;

        let internal_worker = InternalProcessor::new_inlet(receiver, inlet_info);
        ProcessorBuilder::new(internal_worker)
            .with_address(internal_worker_address.clone())
            .with_incoming_access_control(DenyAll)
            .with_outgoing_access_control_arc(options.outgoing_access_control)
            .start(self.ctx())
            .await?;

        Ok(TcpInlet::new_privileged(
            local_address,
            remote_worker_address, // FIXME
            inlet_shared_state,
        ))
    }

    /// Stop the Privileged Inlet
    #[instrument(skip(self), fields(port=port))]
    pub async fn stop_privileged_inlet(&self, port: Port) -> Result<()> {
        self.ebpf_support.inlet_registry.delete_inlet(port);

        Ok(())
    }

    /// Create a Privileged Outlet
    #[instrument(skip(self), fields(address = ? address.clone().into(), peer=peer.clone().to_string()))]
    pub async fn create_privileged_outlet(
        &self,
        address: impl Into<Address> + Clone + Debug,
        peer: HostnamePort,
        options: TcpOutletOptions, // FIXME
    ) -> Result<()> {
        Self::check_capabilities()?;

        // Resolve peer address as a host name and port
        tracing::Span::current().record("peer", peer.to_string());

        let remote_worker_address = address.into();
        let internal_worker_address = Address::random_tagged("Ebpf.InternalWorker.Outlet");

        // TODO: eBPF May be good to run resolution every time there is incoming connection, but that
        //  would require also updating the self.ebpf_support.outlet_registry
        let destination = resolve_peer(&peer).await?;

        let dst_ip = match destination.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                return Err(TransportError::ExpectedIPv4Address)?;
            }
        };
        let dst_port = destination.port();

        // Trigger immediate attach
        self.ebpf_support.attach_ebpf_to_all_interfaces().await?;
        // Start periodic updates if needed
        self.ebpf_support
            .attach_ebpf_to_all_interfaces_start_task()
            .await;

        let write_handle = self.start_raw_socket_processor_if_needed().await?;

        options.setup_flow_control_for_outlet_listener(
            self.ctx().flow_controls(),
            &remote_worker_address,
        );

        let (sender, receiver) = channel(20); // FIXME

        let outlet_info = self.ebpf_support.outlet_registry.add_outlet(
            remote_worker_address.clone(),
            internal_worker_address.clone(),
            sender,
            dst_ip,
            dst_port,
        );

        let remote_worker =
            RemoteWorker::new_outlet(write_handle, outlet_info.clone(), self.ebpf_support.clone());
        WorkerBuilder::new(remote_worker)
            .with_address(remote_worker_address)
            .with_incoming_access_control_arc(options.incoming_access_control)
            .with_outgoing_access_control(DenyAll)
            .start(self.ctx())
            .await?;

        let internal_worker = InternalProcessor::new_outlet(receiver, outlet_info);
        ProcessorBuilder::new(internal_worker)
            .with_address(internal_worker_address)
            .with_incoming_access_control(DenyAll)
            .with_outgoing_access_control_arc(options.outgoing_access_control)
            .start(self.ctx())
            .await?;

        Ok(())
    }

    /// Stop the Privileged Inlet
    #[instrument(skip(self), fields(address = % addr.clone().into()))]
    pub async fn stop_privileged_outlet(
        &self,
        addr: impl Into<Address> + Clone + Debug,
    ) -> Result<()> {
        self.ctx().stop_worker(addr).await?;

        // TODO: eBPF Remove from the registry
        // self.ebpf_support.outlet_registry

        Ok(())
    }
}
