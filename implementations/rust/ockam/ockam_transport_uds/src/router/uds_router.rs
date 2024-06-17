use core::ops::Deref;
use ockam_core::{
    async_trait, compat::sync::Arc, Address, AllowAll, Any, Decodable, LocalMessage, Mailbox,
    Mailboxes, Result, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::TransportError;
use std::collections::BTreeMap;
use tracing::{debug, error, trace};

use super::{UdsRouterHandle, UdsRouterRequest, UdsRouterResponse};
use crate::{address_from_socket_addr, workers::UdsSendWorker, UDS};

/// A UDS address router and connection listener
///
/// In order to create a new UDS connection workers you need a router to
/// map remote addresses of `type = 5` to worker addresses. This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
pub(crate) struct UdsRouter {
    ctx: Context,
    main_addr: Address,
    api_addr: Address,
    map: BTreeMap<Address, Address>,
    allow_auto_connection: bool,
}

/// Public Implementations to instantiate a UDS Router and UDS Router Handler
impl UdsRouter {
    /// Create and register a new UDS router with the given node context
    pub async fn register(ctx: &Context) -> Result<UdsRouterHandle> {
        // This context is only used to start workers, doesn't need to send nor receive messages
        let mailboxes = Mailboxes::new(
            Mailbox::deny_all(Address::random_tagged("UdsRouter.detached")),
            vec![],
        );

        let child_ctx = ctx.new_detached_with_mailboxes(mailboxes).await?;

        let main_addr = Address::random_tagged("UdsRouter_main_addr");
        let api_addr = Address::random_tagged("UdsRouter_api_addr");
        debug!("Initializing new UdsRouter with address {}", &main_addr);

        let router = Self {
            ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
            allow_auto_connection: true,
        };

        let handle = router.create_self_handle().await?;
        let main_mailbox = Mailbox::new(main_addr.clone(), Arc::new(AllowAll), Arc::new(AllowAll));
        let api_mailbox = Mailbox::new(api_addr.clone(), Arc::new(AllowAll), Arc::new(AllowAll));

        WorkerBuilder::new(router)
            .with_mailboxes(Mailboxes::new(main_mailbox, vec![api_mailbox]))
            .start(ctx)
            .await?;

        trace!("Registering UDS router for type = {}", UDS);
        ctx.register(UDS, main_addr).await?;

        Ok(handle)
    }

    /// Create a new [`UdsRouterHandle`] representing this router
    pub async fn create_self_handle(&self) -> Result<UdsRouterHandle> {
        let mailboxes = Mailboxes::new(
            Mailbox::deny_all(Address::random_tagged("UdsRouterHandle.detached")),
            vec![],
        );

        let handle_ctx = self.ctx.new_detached_with_mailboxes(mailboxes).await?;

        let handle =
            UdsRouterHandle::new(handle_ctx, self.main_addr.clone(), self.api_addr.clone());

        Ok(handle)
    }
}

/// Router Handlers Implementations
impl UdsRouter {
    /// Handles any [`UdsRouterRequest::Connect`] messages received by
    /// this node's worker
    async fn handle_connect(&mut self, peer: String) -> Result<Address> {
        let (peer_addr, pathnames) = UdsRouterHandle::resolve_peer(peer)?;

        let router_handle = self.create_self_handle().await?;
        let pair =
            UdsSendWorker::start_pair(&self.ctx, router_handle, None, peer_addr, pathnames.clone())
                .await?;

        let path = match pair.peer().as_pathname() {
            Some(p) => p,
            None => return Err(TransportError::InvalidAddress)?,
        };

        let str_path = match path.to_str() {
            Some(s) => s,
            None => return Err(TransportError::InvalidAddress)?,
        };

        let uds_address = Address::new_with_string(UDS, str_path);
        let mut accepts = vec![uds_address];
        accepts.extend(pathnames.iter().map(|p| Address::new_with_string(UDS, p)));

        let self_addr = pair.tx_addr();
        self.handle_register(accepts, self_addr.clone()).await?;

        Ok(self_addr)
    }

    /// Handles any [`UdsRouterRequest::Disconnect`] messages received by
    /// this node's worker
    async fn handle_disconnect(&mut self, peer: String) -> Result<()> {
        let (peer_sock_addr, _) = UdsRouterHandle::resolve_peer(peer)?;
        let udp_address = address_from_socket_addr(&peer_sock_addr)?;

        let self_address = if let Some(self_address) = self.map.get(&udp_address) {
            self_address.clone()
        } else {
            error!("Failed to disconnect, peer not found: {}", udp_address);
            return Err(TransportError::PeerNotFound)?;
        };

        self.handle_unregister(self_address.clone()).await?;

        self.ctx.stop_worker(self_address).await?;

        Ok(())
    }

    /// Handles any [`UdsRouterRequest::Register`] messages received by
    /// this node's worker
    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        if accepts.is_empty() {
            error!("UDS registration request failed due to an invalid address list. Please provide at least one valid Address.");
        }

        let duplicate_addrs: Vec<String> = self
            .map
            .iter()
            .filter_map(|(addr, _)| {
                if self.map.contains_key(addr) {
                    Some(addr.to_string())
                } else {
                    None
                }
            })
            .collect();

        if !duplicate_addrs.is_empty() {
            error!(
                "UDS Registration request failed, the following addresses were already connected: {}",
                duplicate_addrs.join("\n")
            );
            return Err(TransportError::AlreadyConnected)?;
        }

        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    /// Handle any [`UdsRouterRequest::Unregister`] messages received by
    /// this node's worker
    async fn handle_unregister(&mut self, self_addr: Address) -> Result<()> {
        trace!("UDS unregistration request: {}", &self_addr);

        self.map.retain(|_, v| v != &self_addr);

        Ok(())
    }

    /// Handle any messages sent to the `main` [`Mailbox`] received by this
    /// nodes worker
    async fn handle_route(&mut self, ctx: &Context, msg: LocalMessage) -> Result<()> {
        trace!("UDS route request: {:?}", msg.next_on_onward_route()?);

        // Get the next hop
        let onward = msg.next_on_onward_route()?;

        // Resolve route to the connection worker responsible for the next hop
        let next = self.resolve_route(&onward).await?;

        // Modify the transport message route
        let msg = msg.replace_front_onward_route(&next)?;

        // Send the local message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }
}

impl UdsRouter {
    /// Resolve the route to the provided noward address
    async fn resolve_route(&mut self, onward: &Address) -> Result<Address> {
        // Check if the connection already exists
        if let Some(n) = self.map.get(onward) {
            return Ok(n.clone());
        }

        let peer =
            String::from_utf8(onward.deref().clone()).map_err(|_| TransportError::UnknownRoute)?;
        let (peer_addr, _pathnames) = UdsRouterHandle::resolve_peer(peer.clone())?;

        let path = match peer_addr.as_pathname() {
            Some(p) => p,
            None => {
                error!("Failed to resolve route.");
                return Err(TransportError::InvalidAddress)?;
            }
        };

        let path_str = match path.to_str() {
            Some(s) => s,
            None => {
                error!(
                    "Failed to resolve route, invalid path provided: {}",
                    path.display()
                );
                return Err(TransportError::InvalidAddress)?;
            }
        };

        let uds_address = Address::new_with_string(UDS, path_str);

        if let Some(n) = self.map.get(&uds_address).cloned() {
            return Ok(n);
        }

        if self.allow_auto_connection {
            self.handle_connect(peer).await
        } else {
            error!(
                "Failed to resolve route, no existing connection to peer: {}",
                peer
            );
            Err(TransportError::UnknownRoute)?
        }
    }
}

#[async_trait]
impl Worker for UdsRouter {
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
            self.handle_route(ctx, msg.into_local_message()).await?;
        } else if msg_addr == self.api_addr {
            let msg = UdsRouterRequest::decode(msg.payload())?;
            match msg {
                UdsRouterRequest::Register { accepts, self_addr } => {
                    let res = self.handle_register(accepts, self_addr).await;

                    ctx.send(return_route, UdsRouterResponse::Register(res))
                        .await?;
                }
                UdsRouterRequest::Connect { peer } => {
                    let res = self.handle_connect(peer).await;

                    ctx.send(return_route, UdsRouterResponse::Connect(res))
                        .await?;
                }
                UdsRouterRequest::Disconnect { peer } => {
                    let res = self.handle_disconnect(peer).await;

                    ctx.send(return_route, UdsRouterResponse::Disconnect(res))
                        .await?;
                }
                UdsRouterRequest::Unregister { self_addr } => {
                    let res = self.handle_unregister(self_addr).await;

                    ctx.send(return_route, UdsRouterResponse::Unregister(res))
                        .await?;
                }
            };
        } else {
            error!(
                "UDS router received a message for an invalid address: {}",
                msg_addr
            );
            return Err(TransportError::InvalidAddress)?;
        }

        Ok(())
    }
}
