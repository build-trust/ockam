use crate::tcp::traits::{EndpointResolver, NoConnector};
use crate::tcp::workers::TcpSendWorker;
use crate::TransportError;
use core::fmt::Display;
use core::ops::Deref;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Processor, Result};
use ockam_node::Context;
use tracing::{debug, trace};

use crate::tcp::router::TcpRouterHandle;
use crate::tcp::traits::{IntoSplit, TcpAccepter, TcpBinder};

/// A TCP Processor that listen for new connections and launch a new [WorkerPair](super::sender::WorkerPair) for each one.
///
/// The new Pair will be then registered with a `Router`.
pub struct TcpListenProcessor<T, E> {
    inner: T,
    router_handle: TcpRouterHandle<E>,
    cluster_name: &'static str,
}

impl<T, E, V> TcpListenProcessor<T, E>
where
    T: TcpAccepter + Send + Sync + 'static,
    T::Stream: IntoSplit + Send + Sync + 'static,
    T::Peer: Send + Sync + Clone + Display + 'static,
    <T::Stream as IntoSplit>::ReadHalf: Send + Unpin,
    <T::Stream as IntoSplit>::WriteHalf: Send + Unpin,
    E: EndpointResolver + Send + Sync + 'static,
    E::Hostnames: Deref<Target = [V]>,
    E::Peer: Send + Sync + Clone + Display,
    V: Display,
{
    /// Creates and start a new [TcpListenProcessor].
    pub(crate) async fn start<A, B>(
        ctx: &Context,
        router_handle: TcpRouterHandle<E>,
        addr: A,
        binder: B,
        cluster_name: &'static str,
    ) -> Result<()>
    where
        B: TcpBinder<A, Listener = T>,
        A: Display,
    {
        let waddr = Address::random(0);

        debug!("Binding TcpListener to {}", addr);
        let inner = binder.bind(addr).await.map_err(TransportError::from)?;
        let worker = Self {
            inner,
            router_handle,
            cluster_name,
        };

        ctx.start_processor(waddr, worker).await?;
        Ok(())
    }
}

#[async_trait]
impl<T, E, V> Processor for TcpListenProcessor<T, E>
where
    T: Send + TcpAccepter + Sync + 'static,
    T::Peer: Send + Display + Sync + Clone + 'static,
    T::Stream: IntoSplit + Send + Sync + 'static,
    <T::Stream as IntoSplit>::ReadHalf: Send + Unpin,
    <T::Stream as IntoSplit>::WriteHalf: Send + Unpin,
    E: EndpointResolver + Send + Sync + 'static,
    E::Hostnames: Deref<Target = [V]>,
    V: Display,
    E::Peer: Send + Display + Sync + Clone + 'static,
{
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(self.cluster_name).await
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        trace!("Waiting for incoming TCP connection...");

        // Wait for an incoming connection
        let (stream, peer) = self.inner.accept().await.map_err(TransportError::from)?;

        let empty: &'static [&'static str] = &[];
        // And spawn a connection worker for it
        // TODO: Here stream_connector is not really needed, in fact it's never needed when stream is passed
        // Reflecting that in the API will ease the use of TcpSendWorker.
        let pair = TcpSendWorker::start_pair(
            ctx,
            Some(stream),
            NoConnector(core::marker::PhantomData::<T::Stream>),
            peer,
            empty,
            self.cluster_name,
        )
        .await?;

        // Register the connection with the local TcpRouter
        self.router_handle.register(&pair).await?;

        Ok(true)
    }
}
