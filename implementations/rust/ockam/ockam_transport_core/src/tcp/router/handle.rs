use crate::tcp::traits::{EndpointResolver, IntoSplit, TcpAccepter, TcpBinder, TcpStreamConnector};
use crate::tcp::workers::{TcpListenProcessor, TcpSendWorker, WorkerPair};
use crate::TCP;
use core::fmt::Display;
use core::marker::PhantomData;
use core::ops::Deref;
use ockam_core::{async_trait, RouterMessage};
use ockam_core::{Address, AsyncTryClone, Result};
use ockam_node::Context;

// We need alloc here for `async_trait` it's very easy to prevent this by making this clone
// directly inside of `bind` but since `processors` already needs to be an async_trait and
// ockam_core still needs alloc I'll leave this here.
use ockam_core::compat::boxed::Box;

/// A handle that can be used to easily connect with a new peer with the router.
///
/// Dropping this is harmless.
pub struct TcpRouterHandle<E> {
    ctx: Context,
    addr: Address,
    _endpoint_resolver: PhantomData<E>,
}

#[async_trait]
impl<E, V> AsyncTryClone for TcpRouterHandle<E>
where
    E: EndpointResolver + Send + Sync,
    E::Hostnames: Deref<Target = [V]>,
    E::Peer: Clone + Display + Send + Sync + 'static,
    V: Display,
{
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_context(Address::random(0)).await?;
        Ok(Self::new(child_ctx, self.addr.clone()))
    }
}

impl<E, V> TcpRouterHandle<E>
where
    E: EndpointResolver + Send + Sync + 'static,
    E::Hostnames: Deref<Target = [V]>,
    E::Peer: Clone + Display + Send + Sync + 'static,
    V: Display,
{
    pub async fn bind<A, B>(&self, bind_addr: A, binder: B) -> Result<()>
    where
        B: TcpBinder<A> + Send,
        B::Listener: Send + Sync + 'static,
        <B::Listener as TcpAccepter>::Stream: IntoSplit + Send + Sync,
        <<B::Listener as TcpAccepter>::Stream as IntoSplit>::ReadHalf: Send + Unpin,
        <<B::Listener as TcpAccepter>::Stream as IntoSplit>::WriteHalf: Send + Unpin,
        <B::Listener as TcpAccepter>::Peer: Send + Sync + Clone + Display,
        A: Display,
    {
        TcpListenProcessor::start(
            &self.ctx,
            self.async_try_clone().await?,
            bind_addr,
            binder,
            crate::CLUSTER_NAME,
        )
        .await
    }
}

impl<E, V> TcpRouterHandle<E>
where
    E: EndpointResolver + Send + Sync,
    E::Hostnames: Deref<Target = [V]>,
    E::Peer: Clone + Display + Send + Sync + 'static,
    V: Display,
{
    pub(crate) fn new(ctx: Context, addr: Address) -> Self {
        Self {
            ctx,
            addr,
            _endpoint_resolver: PhantomData,
        }
    }

    /// Returns the context of the router.
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Returns the address of the router.
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    /// Registers the passed pair using accepting from the address format `<PROTOCOL>#<peer>` where PROTOCOL is a universal constant identifying the protocol used and `peer` is either the address of hostname of the peer of the given pair.
    pub(crate) async fn register<T, U, W>(
        &self,
        pair: &WorkerPair<T, U>,
    ) -> Result<(), ockam_core::Error>
    where
        T: Deref<Target = [W]>,
        W: Display,
        U: Clone + Display,
    {
        // definition on how to create the used address for the pair.
        // Note: not using a lambda to be able to be generic over addr.
        fn get_addr(addr: impl Display) -> Address {
            format!("{}#{}", TCP, addr).into()
        }

        // define all external addresses for the pair
        let tcp_address: Address = get_addr(pair.peer());
        let accepts = pair
            .hostnames()
            .iter()
            .map(get_addr)
            .chain(core::iter::once(tcp_address))
            .collect();

        // internal address of the pair
        let self_addr = pair.tx_addr();

        // Send registration request to router address(Note: the implementation needs to ensure that this message is correctly supported)
        self.ctx
            .send(
                self.addr.clone(),
                RouterMessage::Register { accepts, self_addr },
            )
            .await
    }

    /// Creates a new pair connection into peer and registers it with the router.
    pub async fn connect<P, S>(&self, peer: P, stream_connector: S) -> Result<()>
    where
        P: AsRef<str>,
        S: TcpStreamConnector<E::Peer> + Send + Sync + 'static,
        <S::Stream as IntoSplit>::ReadHalf: Send + Unpin + 'static,
        <S::Stream as IntoSplit>::WriteHalf: Send + Unpin + 'static,
    {
        let (endpoint, hostnames) = E::resolve_endpoint(peer.as_ref())?;
        let pair = TcpSendWorker::start_pair::<S::Stream, _>(
            &self.ctx,
            None,
            stream_connector,
            endpoint,
            hostnames,
            crate::CLUSTER_NAME,
        )
        .await?;

        self.register(&pair).await?;

        Ok(())
    }
}
