use crate::ebpf_portal::OutletListenerWorker;
use crate::portal::InletSharedState;
use crate::{TcpInlet, TcpInletOptions, TcpOutletOptions, TcpTransport};
use core::fmt::Debug;
use ockam_core::{Address, DenyAll, Result, Route};
use ockam_node::compat::asynchronous::resolve_peer;
use ockam_node::WorkerBuilder;
use ockam_transport_core::HostnamePort;
use std::net::{IpAddr, SocketAddrV4};
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tracing::instrument;

impl TcpTransport {
    /// Create a Raw Inlet
    #[instrument(skip(self), fields(outlet_route=?outlet_route.clone()))]
    pub async fn create_raw_inlet(
        &self,
        bind_addr: impl Into<String> + Clone + Debug,
        outlet_route: impl Into<Route> + Clone + Debug,
        options: TcpInletOptions,
    ) -> Result<TcpInlet> {
        // TODO: eBPF Find correlation between bind_addr and iface?
        let bind_addr = bind_addr.into();
        let tcp_listener = TcpListener::bind(bind_addr.clone()).await.unwrap(); // FIXME eBPF

        let local_address = tcp_listener.local_addr().unwrap(); // FIXME eBPF
        let ip = match local_address.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                panic!() // FIXME eBPF
            }
        };
        let port = local_address.port();

        let ifaddrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in ifaddrs {
            let addr = match ifaddr.address {
                Some(addr) => addr,
                None => continue,
            };

            let addr = match addr.as_sockaddr_in() {
                Some(addr) => *addr,
                None => continue,
            };

            let addr = SocketAddrV4::from(addr);

            if &ip == addr.ip() || ip.is_unspecified() {
                // TODO: eBPF Should we instead attach to all interfaces & run a periodic task
                //  to identify network interfaces change?
                self.attach_ebpf_if_needed(ifaddr.interface_name)?;
            }
        }

        let _write_handle = self.start_raw_socket_processor_if_needed().await?;

        let inlet_shared_state = Arc::new(RwLock::new(InletSharedState {
            route: outlet_route.into(),
            is_paused: false,
        }));

        self.ebpf_support.inlet_registry.create_inlet(
            options,
            local_address.port(),
            tcp_listener,
            inlet_shared_state.clone(),
        );

        self.ebpf_support.add_inlet_port(port)?;

        Ok(TcpInlet::new_ebpf(local_address, inlet_shared_state))
    }

    /// Stop the Raw Inlet
    #[instrument(skip(self), fields(port=port))]
    pub async fn stop_raw_inlet(&self, port: u16) -> Result<()> {
        self.ebpf_support.inlet_registry.delete_inlet(port);

        Ok(())
    }

    /// Create a Raw Outlet
    #[instrument(skip(self), fields(address = ? address.clone().into(), peer=peer.clone().to_string()))]
    pub async fn create_raw_outlet(
        &self,
        address: impl Into<Address> + Clone + Debug,
        peer: HostnamePort,
        options: TcpOutletOptions,
    ) -> Result<()> {
        // Resolve peer address as a host name and port
        tracing::Span::current().record("peer", peer.to_string());

        let address = address.into();

        // TODO: eBPF May be good to run resolution every time there is incoming connection, but that
        //  would require also updating the self.ebpf_support.outlet_registry
        let destination = resolve_peer(peer.to_string()).await?;

        let dst_ip = match destination.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => {
                // FIXME eBPF
                panic!()
            }
        };
        let dst_port = destination.port();

        // TODO: eBPF Figure out which ifaces might be used and only attach to them
        // TODO: eBPF Should we indeed attach to all interfaces & run a periodic task
        //  to identify network interfaces change?
        let ifaddrs = nix::ifaddrs::getifaddrs().unwrap();
        for ifaddr in ifaddrs {
            let addr = match ifaddr.address {
                Some(addr) => addr,
                None => continue,
            };

            if addr.as_sockaddr_in().is_none() {
                continue;
            };

            self.attach_ebpf_if_needed(ifaddr.interface_name)?;
        }

        let write_handle = self.start_raw_socket_processor_if_needed().await?;

        let access_control = options.incoming_access_control.clone();

        options.setup_flow_control_for_outlet_listener(self.ctx().flow_controls(), &address);

        let outlet_listener_worker = OutletListenerWorker::new(
            options,
            write_handle,
            self.ebpf_support.outlet_registry.clone(),
            dst_ip,
            dst_port,
            self.ebpf_support.clone(),
        );

        WorkerBuilder::new(outlet_listener_worker)
            .with_address(address)
            .with_incoming_access_control_arc(access_control)
            .with_outgoing_access_control(DenyAll)
            .start(self.ctx())
            .await?;

        self.ebpf_support
            .outlet_registry
            .add_outlet(dst_ip, dst_port);

        Ok(())
    }

    /// Stop the Raw Inlet
    #[instrument(skip(self), fields(address = % addr.clone().into()))]
    pub async fn stop_raw_outlet(&self, addr: impl Into<Address> + Clone + Debug) -> Result<()> {
        self.ctx().stop_worker(addr).await?;

        // TODO: eBPF Remove from the registry
        // self.ebpf_support.outlet_registry

        Ok(())
    }
}
