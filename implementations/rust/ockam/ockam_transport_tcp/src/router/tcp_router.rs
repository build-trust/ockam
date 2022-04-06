use crate::{TcpRouterHandle, TcpRouterRequest, TcpRouterResponse, TcpSendWorker, TCP};
use core::ops::Deref;
use ockam_core::{async_trait, Any};
use ockam_core::{Address, Decodable, LocalMessage, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::collections::BTreeMap;
use tracing::{debug, error, trace};

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
    main_addr: Address,
    api_addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
}

impl TcpRouter {
    /// Create and register a new TCP router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`TcpRouter::bind`](TcpRouter::bind)
    pub async fn register(ctx: &Context) -> Result<TcpRouterHandle> {
        let main_addr = Address::random_local();
        let api_addr = Address::random_local();
        debug!("Initialising new TcpRouter with address {}", &main_addr);

        let child_ctx = ctx.new_context(Address::random_local()).await?;

        let router = Self {
            ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
        };

        let handle = router.create_self_handle().await?;

        ctx.start_worker(vec![main_addr.clone(), api_addr], router)
            .await?;
        trace!("Registering TCP router for type = {}", TCP);
        ctx.register(TCP, main_addr).await?;

        Ok(handle)
    }

    /// Create a new `TcpRouterHandle` representing this router
    async fn create_self_handle(&self) -> Result<TcpRouterHandle> {
        let handle_ctx = self.ctx.new_context(Address::random_local()).await?;
        let handle = TcpRouterHandle::new(handle_ctx, self.api_addr.clone());
        Ok(handle)
    }
}

impl TcpRouter {
    /// Handle any [`TcpRouterRequest::Register`] messages received by
    /// this node's worker
    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        if let Some(f) = accepts.first().cloned() {
            trace!("TCP registration request: {} => {}", f, self_addr);
        } else {
            error!("TCP registration request failed due to an invalid address list. Please provide at least one valid Address.");
            return Err(TransportError::InvalidAddress.into());
        }

        for accept in &accepts {
            if self.map.contains_key(accept) {
                error!(
                    "TCP registration request failed, this address is already connected: {}",
                    accept
                );
                return Err(TransportError::AlreadyConnected.into());
            }
        }

        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    /// Handle any [`TcpRouterRequest::Unregister`] messages received by
    /// this node's worker
    async fn handle_unregister(&mut self, self_addr: Address) -> Result<()> {
        trace!("TCP unregistration request: {}", &self_addr);

        self.map.retain(|_, self_addr_i| self_addr_i != &self_addr);

        Ok(())
    }
}

impl TcpRouter {
    /// Handle any [`TcpRouterRequest::Connect`] messages received by this
    /// nodes worker
    ///
    /// This handler starts a `(TcpSendWorker, TcpRecvProcessor)` pair
    /// that open and manage a connection to the given peer and
    /// finally register the given peer with this `TcpRouter`.
    async fn handle_connect(&mut self, peer: String) -> Result<Address> {
        // Resolve peer address
        let (peer_addr, hostnames) = TcpRouterHandle::resolve_peer(peer)?;

        // Start a new `WorkerPair` for the given peer containing a
        // `TcpSendWorker` and `TcpRecvprocessor`
        let router_handle = self.create_self_handle().await?;
        let pair =
            TcpSendWorker::start_pair(&self.ctx, router_handle, None, peer_addr, hostnames.clone())
                .await?;

        // Send this `TcpRouter` a `TcpRouterRequest::Register` message
        // containing the registration request
        let tcp_address = Address::new(TCP, pair.peer().to_string());
        let mut accepts = vec![tcp_address];
        accepts.extend(hostnames.iter().map(|x| Address::new(TCP, x)));
        let self_addr = pair.tx_addr();

        self.handle_register(accepts, self_addr.clone()).await?;

        Ok(self_addr)
    }

    /// Handle any [`TcpRouterRequest::Disconnect`] messages received by this
    /// nodes worker
    async fn handle_disconnect(&mut self, peer: String) -> Result<()> {
        let (peer_addr, _hostnames) = TcpRouterHandle::resolve_peer(peer)?;
        let tcp_address: Address = format!("{}#{}", TCP, peer_addr).into();

        let self_address = if let Some(self_address) = self.map.get(&tcp_address) {
            self_address.clone()
        } else {
            error!("Failed to disconnect, peer not found: {}", tcp_address);
            return Err(TransportError::PeerNotFound.into());
        };

        self.handle_unregister(self_address.clone()).await?;

        self.ctx.stop_worker(self_address).await?;

        Ok(())
    }

    /// Handle any [`RouterMessage::Route`] messages received by this
    /// nodes worker
    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        trace!(
            "TCP route request: {:?}",
            msg.transport().onward_route.next()
        );

        // Get the next hop
        let onward = msg.transport().onward_route.next()?;

        // Resolve route to the connection worker responsible for the next hop
        let next = self.resolve_route(onward).await?;

        // Modify the transport message route
        let _ = msg.transport_mut().onward_route.step()?;
        msg.transport_mut()
            .onward_route
            .modify()
            .prepend(next.clone());

        // Send the transport message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }

    /// Resolve the route to the provided onward address
    async fn resolve_route(&mut self, onward: &Address) -> Result<Address> {
        // Check if the connection already exists
        if let Some(n) = self.map.get(onward) {
            return Ok(n.clone());
        }

        // Try resolve a tcp address for the onward address
        let peer =
            String::from_utf8(onward.deref().clone()).map_err(|_| TransportError::UnknownRoute)?;
        let (peer_addr, hostnames) = TcpRouterHandle::resolve_peer(peer.clone())?;
        let tcp_address = Address::new(TCP, peer_addr.to_string());

        // Check for existing connection under different name
        if let Some(n) = self.map.get(&tcp_address).cloned() {
            // Add new aliases for existing connection
            for accept in hostnames.iter().map(|x| Address::new(TCP, x)) {
                self.map.insert(accept, n.clone());
            }

            return Ok(n);
        }

        // No existing connection
        if self.allow_auto_connection {
            self.handle_connect(peer).await
        } else {
            error!(
                "Failed to resolve route, no existing connection to peer: {}",
                peer
            );
            Err(TransportError::UnknownRoute.into())
        }
    }
}

#[async_trait]
impl Worker for TcpRouter {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let return_route = msg.return_route();
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            let msg = LocalMessage::decode(msg.payload())?;
            self.handle_route(ctx, msg).await?;
        } else if msg_addr == self.api_addr {
            let msg = TcpRouterRequest::decode(msg.payload())?;
            match msg {
                TcpRouterRequest::Register { accepts, self_addr } => {
                    let res = self.handle_register(accepts, self_addr).await;

                    ctx.send(return_route, TcpRouterResponse::Register(res))
                        .await?;
                }
                TcpRouterRequest::Unregister { self_addr } => {
                    let res = self.handle_unregister(self_addr).await;

                    ctx.send(return_route, TcpRouterResponse::Unregister(res))
                        .await?;
                }
                TcpRouterRequest::Connect { peer } => {
                    let res = self.handle_connect(peer).await;

                    ctx.send(return_route, TcpRouterResponse::Connect(res))
                        .await?;
                }
                TcpRouterRequest::Disconnect { peer } => {
                    let res = self.handle_disconnect(peer).await;

                    ctx.send(return_route, TcpRouterResponse::Disconnect(res))
                        .await?;
                }
            };
        } else {
            error!(
                "TCP router received a message for an invalid address: {}",
                msg_addr
            );
            return Err(TransportError::InvalidAddress.into());
        }

        Ok(())
    }
}
