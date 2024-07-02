use crate::portal::addresses::{Addresses, PortalType};
use crate::{portal::TcpPortalWorker, TcpInlet, TcpInletOptions, TcpRegistry};
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, Processor, Result, Route};
use ockam_node::Context;
use ockam_transport_core::{HostnamePort, TransportError};
use tokio::net::TcpListener;
use tracing::{debug, error, instrument};

/// State shared between `TcpInletListenProcessor` and `TcpInlet` to allow manipulating its state
/// from outside the worker: update the route to the outlet or pause it.
#[derive(Debug, Clone)]
pub struct InletSharedState {
    pub route: Route,
    pub is_paused: bool,
}

/// A TCP Portal Inlet listen processor
///
/// TCP Portal Inlet listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_inlet`](crate::TcpTransport::create_inlet).
pub(crate) struct TcpInletListenProcessor {
    registry: TcpRegistry,
    inner: TcpListener,
    outlet_shared_state: Arc<RwLock<InletSharedState>>,
    options: TcpInletOptions,
}

impl TcpInletListenProcessor {
    pub fn new(
        registry: TcpRegistry,
        inner: TcpListener,
        outlet_shared_state: Arc<RwLock<InletSharedState>>,
        options: TcpInletOptions,
    ) -> Self {
        Self {
            registry,
            inner,
            outlet_shared_state,
            options,
        }
    }

    /// Start a new `TcpInletListenProcessor`
    #[instrument(skip_all, name = "TcpInletListenProcessor::start")]
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        outlet_listener_route: Route,
        addr: SocketAddr,
        options: TcpInletOptions,
    ) -> Result<TcpInlet> {
        let processor_address = Address::random_tagged("TcpInletListenProcessor");

        debug!("Binding TcpPortalListenerWorker to {}", addr);
        let inner = match TcpListener::bind(addr).await {
            Ok(addr) => addr,
            Err(err) => {
                error!(%addr, %err, "could not bind to address");
                return Err(TransportError::from(err))?;
            }
        };
        let socket_addr = inner.local_addr().map_err(TransportError::from)?;
        let outlet_shared_state = InletSharedState {
            route: outlet_listener_route,
            is_paused: options.is_paused,
        };
        let outlet_shared_state = Arc::new(RwLock::new(outlet_shared_state));
        let processor = Self::new(registry, inner, outlet_shared_state.clone(), options);

        ctx.start_processor(processor_address.clone(), processor)
            .await?;

        Ok(TcpInlet::new(
            socket_addr,
            processor_address,
            outlet_shared_state,
        ))
    }
}

#[async_trait]
impl Processor for TcpInletListenProcessor {
    type Context = Context;

    #[instrument(skip_all, name = "TcpInletListenProcessor::initialize")]
    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.add_inlet_listener_processor(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpInletListenProcessor::shutdown")]
    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry
            .remove_inlet_listener_processor(&ctx.address());

        Ok(())
    }

    #[instrument(skip_all, name = "TcpInletListenProcessor::process")]
    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let (stream, socket_addr) = self.inner.accept().await.map_err(TransportError::from)?;

        let addresses = Addresses::generate(PortalType::Inlet);

        let outlet_shared_state = self.outlet_shared_state.read().unwrap().clone();

        if outlet_shared_state.is_paused {
            // Just drop the stream
            return Ok(true);
        }

        self.options.setup_flow_control(
            ctx.flow_controls(),
            &addresses,
            outlet_shared_state.route.next()?,
        );

        TcpPortalWorker::start_new_inlet(
            ctx,
            self.registry.clone(),
            stream,
            HostnamePort::try_from(socket_addr)?,
            outlet_shared_state.route,
            addresses,
            self.options.incoming_access_control.clone(),
            self.options.outgoing_access_control.clone(),
        )
        .await?;

        Ok(true)
    }
}
