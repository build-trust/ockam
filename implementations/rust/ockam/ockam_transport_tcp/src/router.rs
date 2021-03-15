use crate::atomic::{self, ArcBool};
use ockam::{async_worker, Address, Context, Result, Routed, RouterMessage, Worker};
use std::{collections::BTreeMap, net::SocketAddr};

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
    pub async fn register(&self, accepts: Address, self_addr: Address) -> Result<()> {
        self.ctx
            .send_message(
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
        ctx.register(1, ctx.address()).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<RouterMessage>,
    ) -> Result<()> {
        let msg = msg.take();
        use RouterMessage::*;
        match msg {
            Route(mut msg) => {
                debug!("TCP route request: {:?}", msg.onward.next());

                // Get the next hop
                let onward = msg.onward.step().unwrap();

                // Look up the connection worker responsible
                let next = self.map.get(&onward).unwrap();

                // Modify the transport message route
                msg.onward.modify().prepend(next.clone());
                msg.return_.modify().prepend(onward);

                // Forward the message to the connection worker
                ctx.forward_message(msg).await?;
            }
            Register { accepts, self_addr } => {
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
    fn create() -> Self {
        Self {
            map: BTreeMap::new(),
            run: atomic::new(true),
        }
    }

    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouter::bind`](TcpRouter::bind)
    pub async fn register<'c>(ctx: &'c Context) -> Result<TcpRouterHandle<'c>> {
        let addr = Address::from("io.ockam.router.tcp");
        let worker = Self::create();

        ctx.start_worker(addr.clone(), worker).await?;
        Ok(TcpRouterHandle { ctx, addr })
    }

    /// Register a new TCP router and bind a connection listener
    ///
    /// Use this function when your node is the server part of your
    /// connection architecture.  For clients that shouldn't listen
    /// for connections themselves, use
    /// [`TcpRouter::register`](TcpRouter::register).
    pub async fn bind<'c, S: Into<SocketAddr>>(
        _ctx: &'c Context,
        _addr: S,
    ) -> Result<TcpRouterHandle<'c>> {
        todo!()
    }
}
