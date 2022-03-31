use core::str::FromStr;
use std::collections::BTreeMap;
use std::ops::Deref;

pub(crate) use handle::WebSocketRouterHandle;
use ockam_core::{
    async_trait, Address, Any, Decodable, LocalMessage, Message, Result, Routed, Worker,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::error::WebSocketError;
use crate::workers::WorkerPair;
use crate::{WebSocketAddress, WS};
use serde::{Deserialize, Serialize};

mod handle;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub(crate) enum WebSocketRouterMessage {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}

/// A WebSocket address router and connection listener
///
/// In order to create new WebSocket connection workers you need a router to
/// map remote addresses of `type = 2` to worker addresses.  This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
/// if the local node is part of a server architecture.
pub(crate) struct WebSocketRouter {
    ctx: Context,
    main_addr: Address,
    api_addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
}

impl WebSocketRouter {
    /// Create and register a new WebSocket router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`WebSocketRouter::bind`](WebSocketRouter::bind)
    pub(crate) async fn register(ctx: &Context) -> Result<WebSocketRouterHandle> {
        let main_addr = Address::random_local();
        let api_addr = Address::random_local();
        debug!(
            "Initialising new WebSocketRouter with address {}",
            &main_addr
        );

        // Create the `WebSocketRouter` instance. Note that both the
        // router and the handle have independent contexts so that
        // they can manage their own lifecycle.
        let child_ctx = ctx.new_context(Address::random_local()).await?;
        let router = Self {
            ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
        };

        let handle = router.create_self_handle(ctx).await?;

        ctx.start_worker(vec![main_addr.clone(), api_addr], router)
            .await?;
        trace!("Registering WS router for type = {}", WS);
        ctx.register(WS, main_addr).await?;

        Ok(handle)
    }

    async fn create_self_handle(&self, ctx: &Context) -> Result<WebSocketRouterHandle> {
        let handle_ctx = ctx.new_context(Address::random_local()).await?;
        let handle = WebSocketRouterHandle::new(handle_ctx, self.api_addr.clone());
        Ok(handle)
    }
}

#[async_trait::async_trait]
impl Worker for WebSocketRouter {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            let msg = LocalMessage::decode(msg.payload())?;
            self.handle_route(ctx, msg).await?;
        } else if msg_addr == self.api_addr {
            let msg = WebSocketRouterMessage::decode(msg.payload())?;
            match msg {
                WebSocketRouterMessage::Register { accepts, self_addr } => {
                    trace!("handle_message register: {:?} => {:?}", accepts, self_addr);
                    self.handle_register(accepts, self_addr).await?;
                }
            };
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok(())
    }
}

impl WebSocketRouter {
    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        trace!(
            "WS route request: {:?}",
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
            let peer_str = match String::from_utf8(onward.deref().clone()) {
                Ok(s) => s,
                Err(_e) => return Err(TransportError::UnknownRoute.into()),
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

    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        // The `accepts` vector should always contain at least one address.
        if let Some(f) = accepts.first().cloned() {
            trace!("WS registration request: {} => {}", f, self_addr);
        }
        // Otherwise, the router is not being used properly and returns an error.
        else {
            error!("Tried to register a new client without passing any `Address`");
            return Err(TransportError::InvalidAddress.into());
        }

        // Do not connect twice.
        for accept in &accepts {
            if self.map.contains_key(accept) {
                return Err(TransportError::AlreadyConnected.into());
            }
        }

        // Add a new entry for each hostname/address pair.
        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    async fn connect(&mut self, peer: String) -> Result<Address> {
        // Get peer address and connect to it.
        let (peer_addr, hostnames) = WebSocketRouterHandle::resolve_peer(peer)?;

        // Create a new `WorkerPair` for the given peer, initializing a new pair
        // of sender worker and receiver processor.
        let pair = WorkerPair::from_client(&self.ctx, peer_addr, hostnames).await?;

        // Handle node's register request.
        let mut accepts = vec![pair.peer()];
        accepts.extend(
            pair.hostnames()
                .iter()
                .filter_map(|x| WebSocketAddress::from_str(x).ok())
                .map(|addr| addr.into()),
        );
        let self_addr = pair.tx_addr();
        self.handle_register(accepts, self_addr.clone()).await?;

        Ok(self_addr)
    }
}
