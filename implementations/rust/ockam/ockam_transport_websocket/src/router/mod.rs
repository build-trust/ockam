use core::str::FromStr;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;

pub(crate) use handle::WebSocketRouterHandle;
use ockam_core::{
    async_trait, Address, AllowAll, Any, Decodable, LocalMessage, Mailbox, Mailboxes, Message,
    Result, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::TransportError;

use crate::workers::WorkerPair;
use crate::{WebSocketAddress, WS};
use serde::{Deserialize, Serialize};

mod handle;

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum WebSocketRouterRequest {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}

#[derive(Serialize, Deserialize, Debug, Message)]
pub enum WebSocketRouterResponse {
    Register(Result<()>),
}

/// A WebSocket address router and connection listener.
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
    /// Create and register a new WebSocket router with the node context.
    pub(crate) fn register(ctx: &Context) -> Result<WebSocketRouterHandle> {
        let main_addr = Address::random_tagged("WebSocketRouter.main_addr");
        let api_addr = Address::random_tagged("WebSocketRouter.api_addr");
        debug!(
            "Initialising new WebSocketRouter with address {}",
            &main_addr
        );

        // Create the `WebSocketRouter` instance. Note that both the
        // router and the handle have independent contexts so that
        // they can manage their own lifecycle.
        // This context is only used to start workers, doesn't need to send nor receive messages
        let mailboxes = Mailboxes::new(
            Mailbox::deny_all(Address::random_tagged("WebSocketRouter.detached")),
            vec![],
        );
        let child_ctx = ctx.new_detached_with_mailboxes(mailboxes)?;
        let router = Self {
            ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
        };

        let handle = router.create_self_handle(ctx)?;

        let mailboxes = Mailboxes::new(
            Mailbox::new(
                main_addr.clone(),
                None,
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            ),
            vec![Mailbox::new(
                api_addr,
                None,
                Arc::new(AllowAll), // FIXME: @ac
                Arc::new(AllowAll), // FIXME: @ac
            )],
        );
        WorkerBuilder::new(router)
            .with_mailboxes(mailboxes)
            .start(ctx)?;
        trace!("Registering WS router for type = {}", WS);
        ctx.register(WS, main_addr)?;

        Ok(handle)
    }

    fn create_self_handle(&self, ctx: &Context) -> Result<WebSocketRouterHandle> {
        let mailboxes = Mailboxes::new(
            Mailbox::deny_all(Address::random_tagged("WebSocketRouter.handle")),
            vec![],
        );
        let handle_ctx = ctx.new_detached_with_mailboxes(mailboxes)?;
        let handle = WebSocketRouterHandle::new(handle_ctx, self.api_addr.clone());
        Ok(handle)
    }
}

#[async_trait]
impl Worker for WebSocketRouter {
    type Message = Any;
    type Context = Context;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let return_route = msg.return_route().clone();
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            self.handle_route(ctx, msg.into_local_message()).await?;
        } else if msg_addr == self.api_addr {
            let msg = WebSocketRouterRequest::decode(msg.payload())?;
            match msg {
                WebSocketRouterRequest::Register { accepts, self_addr } => {
                    trace!("handle_message register: {:?} => {:?}", accepts, self_addr);
                    let res = self.handle_register(accepts, self_addr);

                    ctx.send_from_address(
                        return_route,
                        WebSocketRouterResponse::Register(res),
                        self.api_addr.clone(),
                    )
                    .await?;
                }
            };
        } else {
            return Err(TransportError::InvalidAddress(msg_addr.to_string()))?;
        }

        Ok(())
    }
}

impl WebSocketRouter {
    async fn handle_route(&mut self, ctx: &Context, msg: LocalMessage) -> Result<()> {
        trace!("WS route request: {:?}", msg.onward_route().next());

        // Get the next hop
        let onward = msg.onward_route().next()?;

        let next;
        // Look up the connection worker responsible
        if let Some(n) = self.map.get(onward) {
            // Connection already exists
            next = n.clone();
        } else {
            // No existing connection
            let peer_str = match String::from_utf8(onward.deref().clone()) {
                Ok(s) => s,
                Err(_e) => return Err(TransportError::UnknownRoute)?,
            };

            // TODO: Check if this is the hostname and we have existing/pending connection to this IP
            if self.allow_auto_connection {
                next = self.connect(peer_str)?;
            } else {
                return Err(TransportError::UnknownRoute)?;
            }
        }

        let msg = msg.replace_front_onward_route(next.clone())?;

        // Send the transport message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }

    fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        // The `accepts` vector should always contain at least one address.
        if let Some(f) = accepts.first().cloned() {
            trace!("WS registration request: {} => {}", f, self_addr);
        }
        // Otherwise, the router is not being used properly and returns an error.
        else {
            error!("Tried to register a new client without passing any `Address`");
            return Err(TransportError::InvalidAddress(
                accepts
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
                    .to_string(),
            ))?;
        }

        // Do not connect twice.
        for accept in &accepts {
            if self.map.contains_key(accept) {
                return Err(TransportError::AlreadyConnected)?;
            }
        }

        // Add a new entry for each hostname/address pair.
        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    fn connect(&mut self, peer: String) -> Result<Address> {
        // Get peer address and connect to it.
        let (peer_addr, hostnames) = WebSocketRouterHandle::resolve_peer(peer)?;

        // Create a new `WorkerPair` for the given peer, initializing a new pair
        // of sender worker and receiver processor.
        let pair = WorkerPair::from_client(&self.ctx, peer_addr, hostnames)?;

        // Handle node's register request.
        let mut accepts = vec![pair.peer()];
        accepts.extend(
            pair.hostnames()
                .iter()
                .filter_map(|x| WebSocketAddress::from_str(x).ok())
                .map(|addr| addr.into()),
        );
        let self_addr = pair.tx_addr();
        self.handle_register(accepts, self_addr.clone())?;

        Ok(self_addr)
    }
}
