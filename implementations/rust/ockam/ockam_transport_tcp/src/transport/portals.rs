use crate::portal::{InletSharedState, TcpInletListenProcessor};
use crate::{portal::TcpOutletListenWorker, TcpInletOptions, TcpOutletOptions, TcpTransport};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{route, Address, Result, Route};
use ockam_node::Context;
use ockam_transport_core::{parse_socket_addr, HostnamePort};
use tracing::instrument;

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route_path = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route_path, TcpInletOptions::new()).await?;
    /// # tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? bind_addr.clone().into(), outlet_route = ? outlet_route.clone()))]
    pub async fn create_inlet(
        &self,
        bind_addr: impl Into<String> + Clone + Debug,
        outlet_route: impl Into<Route> + Clone + Debug,
        options: TcpInletOptions,
    ) -> Result<TcpInlet> {
        let socket_address = parse_socket_addr(&bind_addr.into())?;
        TcpInletListenProcessor::start(
            &self.ctx,
            self.registry.clone(),
            outlet_route.into(),
            socket_address,
            options,
        )
        .await
    }

    /// Stop inlet at addr
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_inlet("inlet", route, TcpInletOptions::new()).await?;
    /// tcp.stop_inlet("inlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? addr.clone().into()))]
    pub async fn stop_inlet(&self, addr: impl Into<Address> + Clone + Debug) -> Result<()> {
        self.ctx.stop_processor(addr).await?;

        Ok(())
    }

    /// Create Tcp Outlet Listener at address, that connects to peer using Tcp, transforms Ockam Messages
    /// received from Inlet into stream and sends it to peer Tcp stream. Outlet is bidirectional:
    /// Tcp stream received from peer is transformed into Ockam Routable Messages and sent
    /// to Inlet using return route.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpOutletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # use ockam_transport_core::HostnamePort;
    ///
    /// async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", HostnamePort::new("localhost", 9000), TcpOutletOptions::new()).await?;
    /// # tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? address.clone().into(), peer=peer.clone().to_string()))]
    pub async fn create_outlet(
        &self,
        address: impl Into<Address> + Clone + Debug,
        peer: HostnamePort,
        options: TcpOutletOptions,
    ) -> Result<()> {
        TcpOutletListenWorker::start(
            &self.ctx,
            self.registry.clone(),
            address.into(),
            peer,
            options,
        )
        .await?;

        Ok(())
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::{TcpOutletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result};
    /// # use ockam_transport_core::HostnamePort;
    ///
    /// async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx).await?;
    /// tcp.create_outlet("outlet", HostnamePort::new("127.0.0.1", 5000), TcpOutletOptions::new()).await?;
    /// tcp.stop_outlet("outlet").await?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = % addr.clone().into()))]
    pub async fn stop_outlet(&self, addr: impl Into<Address> + Clone + Debug) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}

/// Result of [`TcpTransport::create_inlet`] call.
#[derive(Clone, Debug)]
pub struct TcpInlet {
    socket_address: SocketAddr,
    state: TcpInletState,
    inlet_shared_state: Arc<RwLock<InletSharedState>>,
}

#[derive(Clone, Debug)]
enum TcpInletState {
    Ebpf,
    Regular { processor_address: Address },
}

impl fmt::Display for TcpInlet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.state {
            TcpInletState::Ebpf => {
                write!(f, "Socket: {}. eBPF", self.socket_address)
            }
            TcpInletState::Regular { processor_address } => {
                write!(
                    f,
                    "Socket: {}. Processor address: {}",
                    self.socket_address, processor_address
                )
            }
        }
    }
}

impl TcpInlet {
    /// Constructor
    pub fn new_regular(
        socket_address: SocketAddr,
        processor_address: Address,
        inlet_shared_state: Arc<RwLock<InletSharedState>>,
    ) -> Self {
        Self {
            socket_address,
            state: TcpInletState::Regular { processor_address },
            inlet_shared_state,
        }
    }

    /// Constructor
    pub fn new_ebpf(
        socket_address: SocketAddr,
        inlet_shared_state: Arc<RwLock<InletSharedState>>,
    ) -> Self {
        Self {
            socket_address,
            state: TcpInletState::Ebpf,
            inlet_shared_state,
        }
    }

    /// Socket Address
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address
    }

    /// Processor address
    pub fn processor_address(&self) -> Option<&Address> {
        match &self.state {
            TcpInletState::Ebpf => None,
            TcpInletState::Regular { processor_address } => Some(processor_address),
        }
    }

    fn build_new_full_route(new_route: Route, old_route: &Route) -> Result<Route> {
        let their_outlet_address = old_route.recipient()?;
        Ok(route![new_route, their_outlet_address])
    }

    /// Update the route to the outlet node.
    /// This is useful if we re-create a secure channel if because, e.g., the other node wasn't
    /// reachable, or if we want to switch transport, e.g., from relayed to UDP NAT puncture.
    ///  NOTE: Existing TCP connections will still use the old route,
    ///        only newly accepted connections will use the new route.
    pub fn update_outlet_node_route(&self, new_route: Route) -> Result<()> {
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();

        inlet_shared_state.route =
            Self::build_new_full_route(new_route, &inlet_shared_state.route)?;

        Ok(())
    }

    /// Pause TCP Inlet, all incoming TCP streams will be dropped.
    pub fn pause(&self) {
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();

        inlet_shared_state.is_paused = true;
    }

    /// Unpause TCP Inlet and update the outlet route.
    pub fn unpause(&self, new_route: Route) -> Result<()> {
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();

        inlet_shared_state.route =
            Self::build_new_full_route(new_route, &inlet_shared_state.route)?;
        inlet_shared_state.is_paused = false;

        Ok(())
    }

    /// Stop the Inlet
    pub async fn stop(&self, ctx: &Context) -> Result<()> {
        match &self.state {
            TcpInletState::Ebpf => {
                // TODO: eBPF
            }
            TcpInletState::Regular { processor_address } => {
                ctx.stop_processor(processor_address.clone()).await?;
            }
        }

        Ok(())
    }
}
