use ockam_core::lib::net::{SocketAddr, ToSocketAddrs};
use ockam_core::lib::{BTreeMap, Deref};
use ockam_core::{worker, Address, LocalMessage, Result, Routed, RouterMessage, Worker};
use ockam_node::Context;
use tracing::{debug, trace};

use crate::common::addr::parse_socket_addr;
use crate::common::connection_listener::ConnectionListenerWorker;
use crate::common::node::TransportNode;
use crate::common::TransportError;

pub struct Router {
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
    pending_connections: Vec<String>,
    addr_id: u8,
}

impl Router {
    pub async fn new(ctx: &Context, addr_id: u8, addr: Address) -> Result<RouterHandler> {
        debug!("Initializing Router instance");
        let router = Self {
            map: BTreeMap::default(),
            allow_auto_connection: true,
            pending_connections: vec![],
            addr_id,
        };
        let handler = router.new_handler(ctx).await?;
        ctx.start_worker(addr.clone(), router).await?;
        Ok(handler)
    }

    async fn new_handler(&self, ctx: &Context) -> Result<RouterHandler> {
        let ctx = ctx.new_context(Address::random(0)).await?;
        // TODO RouterHandler::new(ctx, self.addr.clone())
        RouterHandler::new(ctx, Address::random(0))
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
            let accept_str = accept.to_string();
            self.map.insert(accept.clone(), self_addr.clone());
            // Remove value from pending_connections list
            self.pending_connections.retain(|x| x != &accept_str);
        }

        Ok(())
    }

    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        trace!("Route request: {:?}", msg.transport().onward_route.next());

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
                return Err(TransportError::UnknownRoute.into());
            }

            // TODO: Check if this is the hostname and we have existing/pending connection to this IP

            let peer_addr_str = format!("{}#{}", self.addr_id, &peer_str);
            if self.pending_connections.contains(&peer_addr_str) {
                // We already trying to connect to this address - Requeue the message
                ctx.forward(msg).await?;
            } else if self.allow_auto_connection {
                // Create connection
                self.pending_connections.push(peer_addr_str);
                let handle = self.new_handler(ctx).await?;
                let _ = handle.connect(peer_str).await?;
                // Requeue the message
                ctx.forward(msg).await?;
            } else {
                return Err(TransportError::UnknownRoute.into());
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

#[worker]
impl Worker for Router {
    type Message = RouterMessage;
    type Context = Context;

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        for (_accepts, self_addr) in self.map.iter() {
            ctx.stop_worker(self_addr.clone()).await?;
        }
        Ok(())
    }

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        trace!("Initializing Router worker [addr_id = {}]", self.addr_id);
        ctx.register(self.addr_id, ctx.address()).await?;
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

pub struct RouterHandler {
    ctx: Context,
    addr: Address,
}

impl RouterHandler {
    fn new(ctx: Context, addr: Address) -> Result<Self> {
        debug!("Initializing RouterHandler instance");
        Ok(Self { ctx, addr })
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.ctx.stop_worker(self.addr.clone()).await
    }

    pub async fn register<TN>(&self, node: &TN) -> Result<()>
    where
        TN: TransportNode,
    {
        let msg = RouterMessage::Register {
            accepts: vec![TN::build_addr(node.peer())],
            self_addr: node.tx_addr(),
        };
        self.ctx.send(self.addr.clone(), msg).await
    }

    pub async fn bind<S, CL, TN>(&self, addr: S) -> Result<()>
    where
        S: Into<SocketAddr>,
        CL: ConnectionListenerWorker<Transport = TN>,
        TN: TransportNode,
    {
        CL::start(&self.ctx, addr.into(), self.addr.clone()).await
    }

    pub async fn connect(&self, peer: impl Into<String>) -> Result<()> {
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
                return Err(TransportError::InvalidAddress.into());
            }

            hostnames = vec![peer_str];
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        // let node = WorkerPair::start(&self.ctx, peer_addr, hostnames).await?;
        // self.register(&node).await?;

        Ok(())
    }
}
