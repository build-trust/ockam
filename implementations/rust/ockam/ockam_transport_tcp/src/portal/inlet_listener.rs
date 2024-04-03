use crate::portal::addresses::{Addresses, PortalType};
use crate::{portal::TcpPortalWorker, HostnamePort, TcpInletOptions, TcpRegistry};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{Address, Processor, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::TcpListener;
use tracing::{debug, error, instrument};

/// A TCP Portal Inlet listen processor
///
/// TCP Portal Inlet listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_inlet`](crate::TcpTransport::create_inlet).
pub(crate) struct TcpInletListenProcessor {
    registry: TcpRegistry,
    inner: TcpListener,
    outlet_listener_route: Route,
    options: TcpInletOptions,
}

impl TcpInletListenProcessor {
    pub fn new(
        registry: TcpRegistry,
        inner: TcpListener,
        outlet_listener_route: Route,
        options: TcpInletOptions,
    ) -> Self {
        Self {
            registry,
            inner,
            outlet_listener_route,
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
    ) -> Result<(SocketAddr, Address)> {
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
        let processor = Self::new(registry, inner, outlet_listener_route, options);

        ctx.start_processor(processor_address.clone(), processor)
            .await?;

        Ok((socket_addr, processor_address))
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
        let addresses = Addresses::generate(PortalType::Inlet);
        let outlet_listener_route = self.outlet_listener_route.clone();

        self.options.setup_flow_control(
            ctx.flow_controls(),
            &addresses,
            outlet_listener_route.next()?,
        );

        let (stream, socket_addr) = self.inner.accept().await.map_err(TransportError::from)?;
        TcpPortalWorker::start_new_inlet(
            ctx,
            self.registry.clone(),
            stream,
            HostnamePort::from_socket_addr(socket_addr)?,
            outlet_listener_route,
            addresses,
            self.options.incoming_access_control.clone(),
        )
        .await?;

        Ok(true)
    }
}
