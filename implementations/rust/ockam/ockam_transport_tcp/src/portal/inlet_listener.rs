use crate::portal::addresses::{Addresses, PortalType};
use crate::portal::tls_certificate::TlsCertificateProvider;
use crate::portal::{ReadHalfMaybeTls, WriteHalfMaybeTls};
use crate::{portal::TcpPortalWorker, TcpInlet, TcpInletOptions, TcpRegistry};
use log::warn;
use ockam_core::compat::net::SocketAddr;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_core::{Address, Processor, Route};
use ockam_node::Context;
use ockam_transport_core::{HostnamePort, TransportError};
use rustls::pki_types::CertificateDer;
use std::io::BufReader;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio_rustls::{TlsAcceptor, TlsStream};
use tracing::{debug, error, instrument};

/// State shared between `TcpInletListenProcessor` and `TcpInlet` to allow manipulating its state
/// from outside the worker: update the route to the outlet or pause it.
#[derive(Debug, Clone)]
pub struct InletSharedState {
    route: Route,
    is_paused: bool,
    // Starts with 0 and increments each time when inlet updates the route to the outlet
    // (e.g. when reconnecting), this will allow outlet to figure out what is the most recent
    // return_route even if messages arrive out-of-order
    route_index: u32,
}

impl InletSharedState {
    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn update_route(&mut self, new_route: Route) {
        self.route = new_route;
        // Overflow here is very unlikely...
        self.route_index += 1;
    }

    pub fn is_paused(&self) -> bool {
        self.is_paused
    }

    pub fn set_is_paused(&mut self, is_paused: bool) {
        self.is_paused = is_paused;
    }

    pub fn route_index(&self) -> u32 {
        self.route_index
    }

    pub fn new(is_paused: bool, route: Route) -> Self {
        Self {
            route,
            is_paused,
            route_index: 0,
        }
    }
}

/// A TCP Portal Inlet listen processor
///
/// TCP Portal Inlet listen processors are created by `TcpTransport`
/// after a call is made to
/// [`TcpTransport::create_inlet`](crate::TcpTransport::create_inlet).
pub(crate) struct TcpInletListenProcessor {
    registry: TcpRegistry,
    inner: TcpListener,
    inlet_shared_state: Arc<RwLock<InletSharedState>>,
    options: TcpInletOptions,
}

impl TcpInletListenProcessor {
    pub fn new(
        registry: TcpRegistry,
        inner: TcpListener,
        inlet_shared_state: Arc<RwLock<InletSharedState>>,
        options: TcpInletOptions,
    ) -> Self {
        Self {
            registry,
            inner,
            inlet_shared_state,
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
        let inlet_shared_state = InletSharedState {
            route: outlet_listener_route,
            is_paused: options.is_paused,
            route_index: 0,
        };
        let inlet_shared_state = Arc::new(RwLock::new(inlet_shared_state));
        let processor = Self::new(registry, inner, inlet_shared_state.clone(), options);

        ctx.start_processor(processor_address.clone(), processor)
            .await?;

        Ok(TcpInlet::new_regular(
            socket_addr,
            processor_address,
            inlet_shared_state,
        ))
    }

    /// Returns a TLS acceptor, in case of failure it retries until the timeout is hit.
    /// The timeout is not a hard limit and may be surpassed.
    async fn create_acceptor(
        context: &Context,
        certificate_provider: &Arc<dyn TlsCertificateProvider>,
        timeout: Duration,
    ) -> Result<TlsAcceptor> {
        let now = Instant::now();

        loop {
            if now.elapsed() > timeout {
                return Err(ockam_core::Error::new(
                    Origin::Transport,
                    Kind::Timeout,
                    "TLS certificated retrieval timed out",
                ));
            }

            let certificate = match certificate_provider.get_certificate(context).await {
                Ok(certificate) => certificate,
                Err(error) => {
                    if error.code().kind == Kind::Timeout {
                        warn!("TLS certificate retrieval timed out. Retrying in 60 seconds.");
                    } else {
                        warn!("Cannot retrieve certificate: {error}. Retrying in 60 seconds.");
                    }
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }
            };

            let chain = {
                let mut reader = BufReader::new(certificate.full_chain_pem.as_slice());
                let chain = rustls_pemfile::certs(&mut reader);
                let chain: std::io::Result<Vec<CertificateDer<'static>>> = chain.collect();
                chain.unwrap()
            };

            let private_key = {
                let mut reader = BufReader::new(certificate.private_key_pem.as_slice());
                let mut private_keys = rustls_pemfile::pkcs8_private_keys(&mut reader);

                match private_keys.next() {
                    Some(Ok(private_key)) => private_key.into(),

                    Some(Err(error)) => {
                        return Err(ockam_core::Error::new(
                            Origin::Transport,
                            Kind::Parse,
                            error,
                        ));
                    }

                    None => {
                        return Err(ockam_core::Error::new(
                            Origin::Transport,
                            Kind::Parse,
                            "No private key found in the provided certificate",
                        ));
                    }
                }
            };

            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(chain, private_key)
                .map_err(|error| ockam_core::Error::new(Origin::Transport, Kind::Parse, error))?;

            return Ok(TlsAcceptor::from(Arc::new(config)));
        }
    }
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2 * 60);

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

        let inlet_shared_state = self.inlet_shared_state.read().unwrap().clone();

        if inlet_shared_state.is_paused {
            // Just drop the stream
            return Ok(true);
        }

        TcpInletOptions::setup_flow_control(
            ctx.flow_controls(),
            &addresses,
            inlet_shared_state.route.next()?,
        );

        let streams = if let Some(certificate_provider) = &self.options.tls_certificate_provider {
            let (rx, tx) = tokio::io::split(TlsStream::from(
                Self::create_acceptor(ctx, certificate_provider, DEFAULT_TIMEOUT)
                    .await?
                    .accept(stream)
                    .await
                    .map_err(|error| {
                        ockam_core::Error::new(Origin::Transport, Kind::Protocol, error)
                    })?,
            ));
            (
                ReadHalfMaybeTls::ReadHalfWithTls(rx),
                WriteHalfMaybeTls::WriteHalfWithTls(tx),
            )
        } else {
            let (rx, tx) = stream.into_split();
            (
                ReadHalfMaybeTls::ReadHalfNoTls(rx),
                WriteHalfMaybeTls::WriteHalfNoTls(tx),
            )
        };

        // TODO: Make sure the connection can't be spoofed by someone having access to that Outlet
        TcpPortalWorker::start_new_inlet(
            ctx,
            self.registry.clone(),
            streams,
            HostnamePort::from(socket_addr),
            inlet_shared_state.route,
            addresses,
            self.options.incoming_access_control.clone(),
            self.options.outgoing_access_control.clone(),
        )
        .await?;

        Ok(true)
    }
}
