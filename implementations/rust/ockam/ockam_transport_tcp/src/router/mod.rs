mod handle;
use crate::{TcpSendWorker, TCP};
use core::ops::Deref;
pub(crate) use handle::*;
use ockam_core::async_trait;
use ockam_core::{Address, LocalMessage, Result, Routed, RouterMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::collections::BTreeMap;
use tracing::{debug, trace};

/// A TCP address router and connection listener
///
/// In order to create new TCP connection workers you need a router to
/// map remote addresses of `type = 1` to worker addresses.  This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
/// if the local node is part of a server architecture.
pub(crate) struct TcpRouter {
    ctx: Context,
    addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
}

impl TcpRouter {
    async fn create_self_handle(&self, ctx: &Context) -> Result<TcpRouterHandle> {
        let handle_ctx = ctx.new_context(Address::random(0)).await?;
        let handle = TcpRouterHandle::new(handle_ctx, self.addr.clone());
        Ok(handle)
    }

    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        if let Some(f) = accepts.first().cloned() {
            trace!("TCP registration request: {} => {}", f, self_addr);
        } else {
            // Should not happen
            return Err(TransportError::InvalidAddress.into());
        }

        for accept in &accepts {
            if self.map.contains_key(accept) {
                return Err(TransportError::AlreadyConnected.into());
            }
        }
        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    async fn connect(&mut self, peer: String) -> Result<Address> {
        let (peer_addr, hostnames) = TcpRouterHandle::resolve_peer(peer)?;

        let pair = TcpSendWorker::start_pair(&self.ctx, None, peer_addr, hostnames).await?;

        let tcp_address: Address = format!("{}#{}", TCP, pair.peer()).into();
        let mut accepts = vec![tcp_address];
        accepts.extend(
            pair.hostnames()
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", TCP, x))),
        );
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
            // No existing connection
            let peer_str;
            if let Ok(s) = String::from_utf8(onward.deref().clone()) {
                peer_str = s;
            } else {
                return Err(TransportError::UnknownRoute.into());
            }

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
}

#[async_trait]
impl Worker for TcpRouter {
    type Context = Context;
    type Message = RouterMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("Registering TCP router for type = {}", TCP);
        ctx.register(TCP, ctx.address()).await?;
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
                self.handle_register(accepts, self_addr).await?;
            }
        };

        Ok(())
    }
}

impl TcpRouter {
    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouter::bind`](TcpRouter::bind)
    pub async fn register(ctx: &Context) -> Result<TcpRouterHandle> {
        let addr = Address::random(0);
        debug!("Initialising new TcpRouter with address {}", &addr);

        let child_ctx = ctx.new_context(Address::random(0)).await?;

        let router = Self {
            ctx: child_ctx,
            addr: addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
        };

        let handle = router.create_self_handle(ctx).await?;

        ctx.start_worker(addr.clone(), router).await?;

        Ok(handle)
    }
}
