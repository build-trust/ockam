use crate::portal::{InletSharedState, TcpInletListenProcessor};
use crate::{portal::TcpOutletListenWorker, TcpInletOptions, TcpOutletOptions, TcpTransport};
use core::fmt;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, RwLock as SyncRwLock};
use ockam_core::flow_control::FlowControls;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use ockam_transport_core::{parse_socket_addr, HostnamePort};
use tracing::{debug, instrument};

impl TcpTransport {
    /// Create Tcp Inlet that listens on bind_addr, transforms Tcp stream into Ockam Routable
    /// Messages and forward them to Outlet using outlet_route. Inlet is bidirectional: Ockam
    /// Messages sent to Inlet from Outlet (using return route) will be streamed to Tcp connection.
    /// Pair of corresponding Inlet and Outlet is called Portal.
    ///
    /// ```rust
    /// use ockam_transport_tcp::{TcpInletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{AllowAll, Result, route, Address};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route_path = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx)?;
    /// let address: Address = "inlet".into();
    /// tcp.create_inlet(address.clone(), route_path, TcpInletOptions::new()).await?;
    /// # tcp.stop_inlet(&address)?;
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
    /// # use ockam_core::{AllowAll, Result, route, Address};
    /// # async fn test(ctx: Context) -> Result<()> {
    /// let route = route!["outlet"];
    ///
    /// let tcp = TcpTransport::create(&ctx)?;
    /// let address: Address = "inlet".into();
    /// tcp.create_inlet(address.clone(), route, TcpInletOptions::new()).await?;
    /// tcp.stop_inlet(&address)?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? address))]
    pub fn stop_inlet(&self, address: &Address) -> Result<()> {
        self.ctx.stop_address(address)?;

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
    /// # use ockam_core::{Address, AllowAll, Result};
    /// # use ockam_transport_core::HostnamePort;
    ///
    /// async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx)?;
    /// let address: Address = "outlet".into();
    /// tcp.create_outlet(address.clone(), HostnamePort::new("localhost", 9000)?, TcpOutletOptions::new())?;
    /// # tcp.stop_outlet(&address)?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = ? address.clone().into(), peer=peer.clone().to_string()))]
    pub fn create_outlet(
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
        )?;

        Ok(())
    }

    /// Stop outlet at addr
    /// ```rust
    /// use ockam_transport_tcp::{TcpOutletOptions, TcpTransport};
    /// # use ockam_node::Context;
    /// # use ockam_core::{Address, AllowAll, Result};
    /// # use ockam_transport_core::HostnamePort;
    ///
    /// async fn test(ctx: Context) -> Result<()> {
    ///
    /// let tcp = TcpTransport::create(&ctx)?;
    /// let address: Address = "outlet".into();
    /// tcp.create_outlet(address.clone(), HostnamePort::new("127.0.0.1", 5000)?, TcpOutletOptions::new())?;
    /// tcp.stop_outlet(&address)?;
    /// # Ok(()) }
    /// ```
    #[instrument(skip(self), fields(address = % address))]
    pub fn stop_outlet(&self, address: &Address) -> Result<()> {
        self.ctx.stop_address(address)
    }
}

/// Result of [`TcpTransport::create_inlet`] call.
#[derive(Clone, Debug)]
pub struct TcpInlet {
    socket_address: SocketAddr,
    inlet_shared_state: Arc<SyncRwLock<InletSharedState>>,
    state: TcpInletState,
}

#[derive(Clone, Debug)]
enum TcpInletState {
    Privileged { portal_worker_address: Address },
    Regular { processor_address: Address },
}

impl fmt::Display for TcpInlet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.state {
            TcpInletState::Privileged {
                portal_worker_address,
            } => {
                write!(
                    f,
                    "Socket: {}. Worker address: {}. Privileged",
                    self.socket_address, portal_worker_address
                )
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
        inlet_shared_state: Arc<SyncRwLock<InletSharedState>>,
    ) -> Self {
        Self {
            socket_address,
            inlet_shared_state,
            state: TcpInletState::Regular { processor_address },
        }
    }

    /// Constructor
    pub fn new_privileged(
        socket_address: SocketAddr,
        portal_worker_address: Address,
        inlet_shared_state: Arc<SyncRwLock<InletSharedState>>,
    ) -> Self {
        Self {
            socket_address,
            inlet_shared_state,
            state: TcpInletState::Privileged {
                portal_worker_address,
            },
        }
    }

    /// Returns true if the Inlet is privileged
    pub fn is_privileged(&self) -> bool {
        matches!(self.state, TcpInletState::Privileged { .. })
    }

    /// Socket Address
    pub fn socket_address(&self) -> SocketAddr {
        self.socket_address
    }

    /// Processor address
    pub fn processor_address(&self) -> Option<&Address> {
        match &self.state {
            TcpInletState::Privileged { .. } => None,
            TcpInletState::Regular { processor_address } => Some(processor_address),
        }
    }

    fn build_new_full_route(new_route: Route, old_route: &Route) -> Result<Route> {
        Ok(new_route + old_route.recipient()?.clone())
    }

    /// Update the route to the outlet node.
    /// This is useful if we re-create a secure channel if because, e.g., the other node wasn't
    /// reachable, or if we want to switch transport, e.g., from relayed to UDP NAT puncture.
    ///  NOTE: For regular Portals existing TCP connections will still use the old route,
    ///        only newly accepted connections will use the new route.
    ///        For privileged Portals old connections can continue work in case the Identifier of the
    ///        Outlet node didn't change
    pub fn update_outlet_node_route(&self, ctx: &Context, new_route: Route) -> Result<()> {
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();

        let new_route = Self::build_new_full_route(new_route, inlet_shared_state.route())?;
        let next = new_route.next()?.clone();
        inlet_shared_state.update_route(ctx, new_route)?;

        self.update_flow_controls(ctx.flow_controls(), next);

        Ok(())
    }

    /// Pause TCP Inlet, all incoming TCP streams will be dropped.
    pub fn pause(&self) {
        debug!(address = %self.socket_address, "pausing inlet");
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();
        inlet_shared_state.set_is_paused(true);
    }

    fn update_flow_controls(&self, flow_controls: &FlowControls, next: Address) {
        match &self.state {
            TcpInletState::Privileged {
                portal_worker_address,
            } => {
                TcpInletOptions::setup_flow_control_for_address(
                    flow_controls,
                    portal_worker_address,
                    &next,
                );
            }
            TcpInletState::Regular { .. } => {}
        }
    }

    /// Unpause TCP Inlet and update the outlet route.
    pub fn unpause(&self, ctx: &Context, new_route: Route) -> Result<()> {
        let mut inlet_shared_state = self.inlet_shared_state.write().unwrap();

        let new_route = Self::build_new_full_route(new_route, inlet_shared_state.route())?;
        let next = new_route.next()?.clone();

        inlet_shared_state.update_route(ctx, new_route)?;
        inlet_shared_state.set_is_paused(false);

        self.update_flow_controls(ctx.flow_controls(), next);

        Ok(())
    }

    /// Stop the Inlet
    pub fn stop(&self, ctx: &Context) -> Result<()> {
        match &self.state {
            TcpInletState::Privileged { .. } => {
                // TODO: eBPF
            }
            TcpInletState::Regular { processor_address } => {
                ctx.stop_address(processor_address)?;
            }
        }

        Ok(())
    }
}
