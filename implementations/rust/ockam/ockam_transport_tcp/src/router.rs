use crate::{
    atomic::{self, ArcBool},
    listener::TcpListenWorker,
    TcpError, WorkerPair,
};
use ockam::{async_worker, Address, Context, Result, Routed, RouterMessage, Worker};
use std::{collections::BTreeMap, net::SocketAddr};

const DEFAULT_ADDRESS: &'static str = "io.ockam.router.tcp";

/// A TCP address router and connection listener
///
/// In order to create new TCP connection workers you need a router to
/// map remote addresses of `type = 1` to worker addresses.  This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
/// if the local node is part of a server architecture.
pub struct TcpRouter {
    map: BTreeMap<Address, Address>,
    run: ArcBool,
}

/// A handle to connect to a TcpRouter
///
/// Dropping this handle is harmless.
pub struct TcpRouterHandle<'c> {
    ctx: &'c Context,
    addr: Address,
}

impl<'c> TcpRouterHandle<'c> {
    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let accepts = format!("1#{}", pair.peer.clone()).into();
        let self_addr = pair.tx_addr.clone();

        self.ctx
            .send(
                self.addr.clone(),
                RouterMessage::Register { accepts, self_addr },
            )
            .await
    }
}

#[async_worker]
impl Worker for TcpRouter {
    type Context = Context;
    type Message = RouterMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("Registering TCP router for type = 1");
        ctx.register(1, ctx.primary_address()).await?;
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
            Route(mut msg) => {
                trace!("TCP route request: {:?}", msg.onward_route.next());

                // Get the next hop
                let onward = msg.onward_route.step()?;

                // Look up the connection worker responsible
                let next = self
                    .map
                    .get(&onward)
                    .ok_or_else(|| TcpError::UnknownRoute)?;

                // Modify the transport message route
                msg.onward_route.modify().prepend(next.clone());

                // Send the transport message to the connection worker
                ctx.send(next.clone(), msg).await?;
            }
            Register { accepts, self_addr } => {
                trace!("TCP registration request: {} => {}", accepts, self_addr);
                self.map.insert(accepts, self_addr);
            }
        };

        Ok(())
    }

    fn shutdown(&mut self, _: &mut Context) -> Result<()> {
        // Shut down the ListeningWorker if it exists
        atomic::stop(&self.run);
        Ok(())
    }
}

impl TcpRouter {
    async fn start(ctx: &Context, waddr: &Address, run: Option<ArcBool>) -> Result<()> {
        debug!("Initialising new TcpRouter with address {}", waddr);

        let router = Self {
            map: BTreeMap::new(),
            run: run.unwrap_or_else(|| atomic::new(true)),
        };
        ctx.start_worker(waddr.clone(), router).await?;
        Ok(())
    }

    /// Either register a new router or return a handle to the existing one
    pub(crate) async fn register_or_get<'c>(ctx: &'c Context) -> Result<TcpRouterHandle<'c>> {
        let addr = Address::from(DEFAULT_ADDRESS);
        Self::register(ctx).await.or_else(|_| {
            debug!("Using pre-existing TCP router...");
            Ok(TcpRouterHandle { ctx, addr })
        })
    }

    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouter::bind`](TcpRouter::bind)
    pub async fn register<'c>(ctx: &'c Context) -> Result<TcpRouterHandle<'c>> {
        let addr = Address::from(DEFAULT_ADDRESS);
        Self::start(ctx, &addr, None).await?;
        Ok(TcpRouterHandle { ctx, addr })
    }

    /// Register a new TCP router and bind a connection listener
    ///
    /// Use this function when your node is the server part of your
    /// connection architecture.  For clients that shouldn't listen
    /// for connections themselves, use
    /// [`TcpRouter::register`](TcpRouter::register).
    pub async fn bind<'c, S: Into<SocketAddr>>(
        ctx: &'c Context,
        socket_addr: S,
    ) -> Result<TcpRouterHandle<'c>> {
        let run = atomic::new(true);
        let addr = Address::from(DEFAULT_ADDRESS);

        // Bind and start the connection listen worker
        TcpListenWorker::start(ctx, addr.clone(), socket_addr.into(), run.clone()).await?;

        Self::start(ctx, &addr, Some(run)).await?;
        Ok(TcpRouterHandle { ctx, addr })
    }
}
