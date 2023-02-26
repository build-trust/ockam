use crate::router::messages::{UdpRouterRequest, UdpRouterResponse};
use crate::router::UdpRouterHandle;
use crate::workers::{TransportMessageCodec, UdpListenProcessor, UdpSendWorker};
use futures_util::StreamExt;
use ockam_core::{
    async_trait, Address, AllowAll, Any, Decodable, DenyAll, LocalMessage, Mailbox, Mailboxes,
    Result, Routed, Worker,
};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::TransportError;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;
use tracing::{debug, error, trace};

/// The router for the UDP transport
///
/// The router opens a single 'client' local socket for messages which were
/// initiaited by an entity within the local node.
///
/// The router opens a 'server' local socket whenever a user calls
/// [`listen()`](crate::UdpTransport::listen) on the transport.
///
/// For each open local socket, the router creates a 'sender'
/// ([`UdpSendWorker`](UdpSendWorker)) and a 'listener'
/// ([`UdpListenProcessor`](UdpListenProcessor)) to handle messages
/// sent and received on that socket.
///
/// The router only expects to have to route 'client' messages to the 'client'
/// sender. 'server' messages bypass the router as listeners inject the
/// sender's address into the return route of received messages.
///
/// This transport only supports IPv4.
pub(crate) struct UdpRouter {
    ctx: Context,
    main_addr: Address,
    api_addr: Address,
    /// Sender for 'client' messages
    client_sender: Address,
}

impl UdpRouter {
    /// Create and register a new UDP router with the node context
    pub(crate) async fn register(ctx: &Context) -> Result<UdpRouterHandle> {
        // This context is only used to start workers, doesn't need to send nor receive messages
        let child_ctx = ctx
            .new_detached(
                Address::random_tagged("UdpRouter.detached"),
                DenyAll,
                DenyAll,
            )
            .await?;

        let main_addr = Address::random_tagged("UdpRouter.main_addr");
        let api_addr = Address::random_tagged("UdpRouter.api_addr");
        debug!("Initialising new UdpRouter with address {}", &main_addr);

        let handle = UdpRouterHandle::try_new(&child_ctx, &api_addr).await?;

        // Create sender, listener pair for 'client' messages
        let client_sender = Self::create_sender_listener(
            &child_ctx,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0),
        )
        .await?;

        let router = Self {
            ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            client_sender,
        };

        let main_mailbox = Mailbox::new(
            main_addr.clone(),
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        );
        let api_mailbox = Mailbox::new(
            api_addr.clone(),
            Arc::new(AllowAll), // FIXME: @ac
            Arc::new(AllowAll), // FIXME: @ac
        );
        WorkerBuilder::with_mailboxes(Mailboxes::new(main_mailbox, vec![api_mailbox]), router)
            .start(ctx)
            .await?;

        trace!("Registering UDP router for type = {}", crate::UDP);
        ctx.register(crate::UDP, main_addr).await?;

        Ok(handle)
    }

    /// Handle the routing of 'client' messages
    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        // Forward message to sender for 'client' messages
        let addr = self.client_sender.clone();
        msg.transport_mut().onward_route.modify().prepend(addr);
        ctx.forward(msg).await
    }

    /// Create a sender, listener pair for the given socket address.
    ///
    /// Returns the address of the created sender.
    async fn create_sender_listener(ctx: &Context, local_addr: SocketAddr) -> Result<Address> {
        // This transport only supports IPv4
        if !local_addr.is_ipv4() {
            error!(local_addr = %local_addr, "This transport only supprts IPv4");
            return Err(TransportError::InvalidAddress.into());
        }

        // Bind new socket
        let socket = UdpSocket::bind(local_addr)
            .await
            .map_err(|_| TransportError::InvalidAddress)?;

        // Split socket into sink and stream
        let (sink, stream) = UdpFramed::new(socket, TransportMessageCodec).split();

        debug!("Creating new sender and listener for {}", local_addr);

        // Create sender
        let sender_addr = Address::random_tagged("UdpSendWorker");
        let sender = UdpSendWorker::new(sink);
        // FIXME: @ac
        ctx.start_worker(sender_addr.clone(), sender, AllowAll, AllowAll)
            .await?;

        // Create listener
        UdpListenProcessor::start(ctx, stream, sender_addr.clone()).await?;

        Ok(sender_addr)
    }
}

#[async_trait]
impl Worker for UdpRouter {
    type Message = Any;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            // Process messages on main_addr
            trace!(
                "handle_message() MAIN_ADDR: onward_route = {}, return_route = {}",
                msg.return_route(),
                msg.onward_route(),
            );
            let msg = msg.into_local_message();
            self.handle_route(ctx, msg).await?;
        } else if msg_addr == self.api_addr {
            // Process messages on api_addr
            let return_route = msg.return_route();
            let msg = UdpRouterRequest::decode(msg.payload())?;
            trace!("handle_message() API_ADDR: msg = {:?}", msg);
            match msg {
                UdpRouterRequest::Listen { local_addr } => {
                    let res = Self::create_sender_listener(&self.ctx, local_addr).await;
                    let res = res.map(|_| ());
                    ctx.send_from_address(return_route, UdpRouterResponse::Listen(res), msg_addr)
                        .await?;
                }
            };
        } else {
            return Err(TransportError::Protocol.into());
        }

        Ok(())
    }
}
