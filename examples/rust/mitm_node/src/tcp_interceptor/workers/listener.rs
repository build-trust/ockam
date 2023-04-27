use crate::tcp_interceptor::{Role, TcpMitmProcessor, TcpMitmRegistry, TcpMitmTransport, CLUSTER_NAME};
use ockam_core::{async_trait, compat::net::SocketAddr, DenyAll};
use ockam_core::{Address, Processor, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

pub(crate) struct TcpMitmListenProcessor {
    inner: TcpListener,
    registry: TcpMitmRegistry,
    tcp: TcpMitmTransport,
    target_addr: SocketAddr,
}

impl TcpMitmListenProcessor {
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpMitmRegistry,
        addr: SocketAddr,
        tcp: TcpMitmTransport,
        target_addr: SocketAddr,
    ) -> Result<(SocketAddr, Address)> {
        debug!("Binding TcpListener to {}", addr);
        let inner = TcpListener::bind(addr).await.map_err(TransportError::from)?;
        let saddr = inner.local_addr().map_err(TransportError::from)?;

        let address = Address::random_tagged("TcpListenProcessor");

        let processor = Self {
            inner,
            registry,
            tcp,
            target_addr,
        };

        ctx.start_processor(address.clone(), processor, DenyAll, DenyAll)
            .await?;

        Ok((saddr, address))
    }
}

#[async_trait]
impl Processor for TcpMitmListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(CLUSTER_NAME).await?;

        self.registry.add_listener(&ctx.address());

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        self.registry.remove_listener(&ctx.address());

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming TCP connection...");

        let (stream, _peer) = self.inner.accept().await.map_err(TransportError::from)?;
        debug!("TCP connection accepted");

        // Connection to the target
        let (target_read_half, target_write_half) = TcpStream::connect(self.target_addr).await.unwrap().into_split();

        // Connection from the source
        let (read_half, write_half) = stream.into_split();

        let address1 = Address::random_tagged("receiver_read_target");
        let address2 = Address::random_tagged("receiver_read_source");

        // Forward from the target connection to the source
        TcpMitmProcessor::start(
            ctx,
            Role::ReadTarget,
            address1.clone(),
            address2.clone(),
            target_read_half,
            write_half,
            self.registry.clone(),
        )
        .await?;

        // Forward from the source connection to the target
        TcpMitmProcessor::start(
            ctx,
            Role::ReadSource,
            address2,
            address1,
            read_half,
            target_write_half,
            self.registry.clone(),
        )
        .await?;

        Ok(true)
    }
}
