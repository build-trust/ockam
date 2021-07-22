use crate::{
    atomic::{self, ArcBool},
    listener::TcpListenWorker,
    TcpError, WorkerPair,
};
use async_trait::async_trait;
use ockam_core::{Address, Result, Routed, RouterMessage, Worker};
use ockam_node::Context;
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

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
pub struct TcpRouterHandle {
    ctx: Context,
    addr: Address,
    run: ArcBool,
}

impl TcpRouterHandle {
    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let tcp_address: Address = format!("{}#{}", crate::TCP, pair.peer.clone()).into();
        let mut accepts = vec![tcp_address];
        accepts.extend(
            pair.hostnames
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", crate::TCP, x))),
        );
        let self_addr = pair.tx_addr.clone();

        self.ctx
            .send(
                self.addr.clone(),
                RouterMessage::Register { accepts, self_addr },
            )
            .await
    }

    /// Bind an incoming connection listener for this router
    pub async fn bind<S: Into<SocketAddr>>(&self, addr: S) -> Result<()> {
        let socket_addr = addr.into();
        TcpListenWorker::start(
            &self.ctx,
            self.addr.clone(),
            socket_addr,
            Arc::clone(&self.run),
        )
        .await
    }
}

#[async_trait]
impl Worker for TcpRouter {
    type Context = Context;
    type Message = RouterMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("Registering TCP router for type = {}", crate::TCP);
        ctx.register(crate::TCP, ctx.address()).await?;
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
                trace!(
                    "TCP route request: {:?}",
                    msg.transport().onward_route.next()
                );

                // Get the next hop
                let onward = msg.transport_mut().onward_route.step()?;

                // Look up the connection worker responsible
                let next = self.map.get(&onward).ok_or(TcpError::UnknownRoute)?;

                // Modify the transport message route
                msg.transport_mut()
                    .onward_route
                    .modify()
                    .prepend(next.clone());

                // Send the transport message to the connection worker
                ctx.send(next.clone(), msg).await?;
            }
            Register { accepts, self_addr } => {
                if let Some(f) = accepts.first().cloned() {
                    trace!("TCP registration request: {} => {}", f, self_addr);
                } else {
                    // Should not happen
                    return Err(TcpError::InvalidAddress.into());
                }

                for accept in &accepts {
                    if self.map.contains_key(accept) {
                        return Err(TcpError::AlreadyConnected.into());
                    }
                }
                for accept in accepts {
                    self.map.insert(accept, self_addr.clone());
                }
            }
        };

        Ok(())
    }

    async fn shutdown(&mut self, _: &mut Context) -> Result<()> {
        // Shut down the ListeningWorker if it exists
        atomic::stop(&self.run);
        Ok(())
    }
}

impl TcpRouter {
    async fn start(ctx: &Context, waddr: &Address, run: ArcBool) -> Result<()> {
        debug!("Initialising new TcpRouter with address {}", waddr);

        let router = Self {
            map: BTreeMap::new(),
            run,
        };
        ctx.start_worker(waddr.clone(), router).await?;
        Ok(())
    }

    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouter::bind`](TcpRouter::bind)
    pub async fn register(ctx: Context, addr: Address) -> Result<TcpRouterHandle> {
        let run = atomic::new(true);
        Self::start(&ctx, &addr, Arc::clone(&run)).await?;
        Ok(TcpRouterHandle { ctx, addr, run })
    }
}
