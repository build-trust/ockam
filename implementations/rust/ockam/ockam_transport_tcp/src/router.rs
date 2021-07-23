use crate::{
    atomic::{self, ArcBool},
    init,
    listener::TcpListenWorker,
    parse_socket_addr, TcpError, WorkerPair, TCP,
};
use async_trait::async_trait;
use ockam_core::lib::net::ToSocketAddrs;
use ockam_core::lib::Deref;
use ockam_core::{Address, LocalMessage, Result, Routed, RouterMessage, ServiceBuilder, Worker};
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
    addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
    pending_connections: Vec<String>,
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
    /// Return Context
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }
}

impl TcpRouterHandle {
    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let tcp_address: Address = format!("{}#{}", TCP, pair.peer.clone()).into();
        let mut accepts = vec![tcp_address];
        accepts.extend(
            pair.hostnames
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", TCP, x))),
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

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect(&self, peer: impl Into<String>) -> Result<ServiceBuilder> {
        let peer_str = peer.into();
        let peer_addr;
        let hostnames;

        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            hostnames = vec![];
        }
        // Try to resolve hostname
        else if let Ok(iter) = peer_str.to_socket_addrs() {
            // FIXME: We only take ipv4 for now
            if let Some(p) = iter.filter(|x| x.is_ipv4()).next() {
                peer_addr = p;
            } else {
                return Err(TcpError::InvalidAddress.into());
            }

            hostnames = vec![peer_str];
        } else {
            return Err(TcpError::InvalidAddress.into());
        }

        let serv_builder = ServiceBuilder::new(TCP, peer_addr.to_string());
        init::start_connection(self, peer_addr, hostnames).await?;
        Ok(serv_builder)
    }
}

impl TcpRouter {
    async fn create_self_handle(&self, ctx: &Context) -> Result<TcpRouterHandle> {
        let handle_ctx = ctx.new_context(Address::random(0)).await?;

        let handle = TcpRouterHandle {
            ctx: handle_ctx,
            addr: self.addr.clone(),
            run: Arc::clone(&self.run),
        };

        Ok(handle)
    }

    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
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
            let accept_str = accept.to_string();
            self.map.insert(accept.clone(), self_addr.clone());
            // Remove value from pending_connections list
            self.pending_connections.retain(|x| x != &accept_str);
        }

        Ok(())
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
        if let Some(n) = self.map.get(&onward) {
            // Connection already exists
            next = n;
        } else {
            // No existing connection
            let peer_str;
            if let Ok(s) = String::from_utf8(onward.deref().clone()) {
                peer_str = s;
            } else {
                return Err(TcpError::UnknownRoute.into());
            }

            // TODO: Check if this is the hostname and we have existing/pending connection to this IP

            let peer_addr_str = format!("{}#{}", TCP, &peer_str);
            if self.pending_connections.contains(&peer_addr_str) {
                // We already trying to connect to this address - Requeue the message
                ctx.forward(msg).await?;
            } else if self.allow_auto_connection {
                // Create connection
                self.pending_connections.push(peer_addr_str);
                let handle = self.create_self_handle(ctx).await?;
                let _ = handle.connect(peer_str).await?;
                // Requeue the message
                ctx.forward(msg).await?;
            } else {
                return Err(TcpError::UnknownRoute.into());
            }

            return Ok(());
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

    async fn shutdown(&mut self, _: &mut Context) -> Result<()> {
        // Shut down the ListeningWorker if it exists
        atomic::stop(&self.run);
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

        let run = atomic::new(true);

        let router = Self {
            addr: addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
            pending_connections: vec![],
            run: Arc::clone(&run),
        };

        let handle = router.create_self_handle(ctx).await?;

        ctx.start_worker(addr.clone(), router).await?;

        Ok(handle)
    }
}
