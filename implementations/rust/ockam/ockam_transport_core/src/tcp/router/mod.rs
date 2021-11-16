//! Routing related structs and methods.
use super::traits::EndpointResolver;
use super::traits::IntoSplit;
use super::traits::TcpStreamConnector;
mod handle;
use super::workers::TcpSendWorker;
use crate::TransportError;
use crate::CLUSTER_NAME;
use crate::TCP;
use core::fmt::Display;
use core::iter::Iterator;
use core::marker::PhantomData;
use core::ops::Deref;
pub use handle::TcpRouterHandle;
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, LocalMessage, Result, Routed, RouterMessage, Worker};
use ockam_node::Context;
use tracing::{debug, trace};

/// A TCP address router and connection listener
///
/// In order to create new TCP connection workers you need a router to
/// map remote addresses of `type = 1` to worker addresses.  This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
/// if the local node is part of a server architecture.
pub struct TcpRouter<T, A, P> {
    ctx: Context,
    addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
    stream_connector: T,
    cluster_name: &'static str,
    _marker: PhantomData<(A, P)>,
}

impl<T, A, P, U> TcpRouter<T, A, P>
where
    T: TcpStreamConnector<A> + Clone + Send + Sync + 'static,
    T::Stream: Send,
    <T::Stream as IntoSplit>::ReadHalf: Send + Unpin + 'static,
    <T::Stream as IntoSplit>::WriteHalf: Send + Unpin + 'static,
    A: Clone + Display + Sync + Send + 'static,
    P: EndpointResolver<Peer = A> + Send + Sync + 'static,
    P::Hostnames: Deref<Target = [U]> + Send + Sync,
    U: Display + Sync,
{
    async fn create_self_handle(&self, ctx: &Context) -> Result<TcpRouterHandle<P>> {
        let handle_ctx = ctx.new_context(Address::random(0)).await?;
        let handle = TcpRouterHandle::new(handle_ctx, self.addr.clone());
        Ok(handle)
    }

    async fn handle_register(
        &mut self,
        accepts: impl Iterator<Item = Address> + Clone,
        self_addr: Address,
    ) -> Result<()> {
        if let Some(f) = accepts.clone().next() {
            trace!("TCP registration request: {} => {}", f, self_addr);
        } else {
            // Should not happen
            return Err(TransportError::InvalidAddress.into());
        }

        for accept in accepts.clone() {
            if self.map.contains_key(&accept) {
                return Err(TransportError::AlreadyConnected.into());
            }
        }
        for accept in accepts {
            self.map.insert(accept, self_addr.clone());
        }

        Ok(())
    }

    async fn connect(&mut self, peer_str: &str) -> Result<Address> {
        let (peer_addr, hostnames) = P::resolve_endpoint(peer_str)?;
        let pair = TcpSendWorker::start_pair::<T::Stream, _>(
            &self.ctx,
            None,
            self.stream_connector.clone(),
            peer_addr,
            hostnames,
            self.cluster_name,
        )
        .await?;

        let tcp_address: Address = format!("{}#{}", TCP, pair.peer()).into();
        let accepts = pair
            .hostnames()
            .iter()
            .map(|h| format!("{}#{}", TCP, h).into())
            .chain(core::iter::once(tcp_address));

        let self_addr = pair.tx_addr();

        self.handle_register(accepts, self_addr.clone()).await?;

        Ok(self_addr)
    }

    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        trace!(
            "TCP route request: {:?}",
            msg.transport().onward_route.next()
        );

        // Get the next hop
        let onward = msg.transport().onward_route.next()?;

        let next;
        // Look up the connection worker responsible
        if let Some(n) = self.map.get(onward) {
            // Connection already exists
            next = n.clone();
        } else {
            trace!("Route not found");
            // No existing connection
            let peer_str = if let Ok(s) = core::str::from_utf8(onward) {
                s
            } else {
                return Err(TransportError::UnknownRoute.into());
            };

            // TODO: Check if this is the hostname and we have existing/pending connection to this IP
            if self.allow_auto_connection {
                next = self.connect(peer_str).await?;
            } else {
                return Err(TransportError::UnknownRoute.into());
            }
        }

        let _ = msg.transport_mut().onward_route.step()?;
        // Modify the transport message route
        msg.transport_mut()
            .onward_route
            .modify()
            .prepend(next.clone());

        // Send the transport message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }

    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouterHandle::bind`](TcpRouterHandle::bind)
    pub async fn register(ctx: &Context, stream_connector: T) -> Result<TcpRouterHandle<P>> {
        let addr = Address::random(0);
        debug!("Initialising new TcpRouter with address {}", &addr);

        let child_ctx = ctx.new_context(Address::random(0)).await?;

        let router = Self {
            ctx: child_ctx,
            addr: addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
            cluster_name: CLUSTER_NAME,
            stream_connector,
            _marker: PhantomData,
        };

        let handle = router.create_self_handle(ctx).await?;

        ctx.start_worker(addr.clone(), router).await?;
        trace!("Registering TCP router for type = {}", TCP);
        ctx.register(TCP, addr).await?;

        Ok(handle)
    }
}

#[async_trait]
impl<T, A, P, U> Worker for TcpRouter<T, A, P>
where
    T: TcpStreamConnector<A> + Clone + Send + Sync + 'static,
    <T::Stream as IntoSplit>::ReadHalf: Send + Unpin + 'static,
    <T::Stream as IntoSplit>::WriteHalf: Send + Unpin + 'static,
    T::Stream: Send,
    A: Clone + Display + Sync + Send + 'static,
    P: EndpointResolver<Peer = A> + Send + Sync + 'static,
    P::Hostnames: Deref<Target = [U]> + Send + Sync,
    U: Display + Sync,
{
    type Context = Context;
    type Message = RouterMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<RouterMessage>,
    ) -> Result<()> {
        let msg = msg.body();
        use RouterMessage::*;
        match msg {
            Route(msg) => {
                self.handle_route(ctx, msg).await?;
            }
            Register { accepts, self_addr } => {
                self.handle_register(accepts.into_iter(), self_addr).await?;
            }
        };

        Ok(())
    }
}
