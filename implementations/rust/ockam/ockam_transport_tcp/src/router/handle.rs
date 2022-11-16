use crate::{
    parse_socket_addr, TcpInletListenProcessor, TcpListenProcessor, TcpRouterRequest,
    TcpRouterResponse, WorkerPair, TCP,
};
use ockam_core::compat::net::{SocketAddr, ToSocketAddrs};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, sync::Arc},
};
use ockam_core::{AccessControl, Address, AsyncTryClone, Mailbox, Mailboxes, Result, Route};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use tracing::debug;

/// A handle to connect to a TcpRouter
///
/// Dropping this handle is harmless.
pub(crate) struct TcpRouterHandle {
    ctx: Context,
    api_addr: Address,
    main_addr: Address,
}

#[async_trait]
impl AsyncTryClone for TcpRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        // TODO @ac 0#TcpRouterHandle.async_try_clone.detached
        // in:  n/a - DenyAll?
        // out: n/a - DenyAll?
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                Address::random_tagged("TcpRouterHandle.async_try_clone.detached"),
                Arc::new(ockam_core::AllowAll),
                Arc::new(ockam_core::AllowAll),
            ),
            vec![],
        );
        let child_ctx = self.ctx.new_detached_with_mailboxes(mailboxes).await?;

        Ok(Self::new(
            child_ctx,
            self.main_addr.clone(),
            self.api_addr.clone(),
        ))
    }
}

impl TcpRouterHandle {
    /// Create a new `TcpRouterHandle` with the given address
    pub(crate) fn new(ctx: Context, main_addr: Address, api_addr: Address) -> Self {
        TcpRouterHandle {
            ctx,
            main_addr,
            api_addr,
        }
    }

    /// Return a reference to the router handle's [`Context`]
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    pub(crate) fn main_addr(&self) -> &Address {
        &self.main_addr
    }
}

impl TcpRouterHandle {
    /// Bind an incoming connection listener for this router
    pub async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<SocketAddr> {
        let socket_addr = addr.into();
        TcpListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

    /// Establish an outgoing TCP connection on an existing transport
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<Address> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                TcpRouterRequest::Connect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        if let TcpRouterResponse::Connect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Disconnect an outgoing TCP connection on an existing transport
    pub async fn disconnect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                TcpRouterRequest::Disconnect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        if let TcpRouterResponse::Disconnect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let tcp_address: Address = format!("{}#{}", TCP, pair.peer()).into();
        let mut accepts = vec![tcp_address];
        accepts.extend(
            pair.hostnames()
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", TCP, x))),
        );
        let self_addr = pair.tx_addr();
        // TODO @ac 0#RegisterConnectionWorker.detached
        // in:   0#RegisterConnectionWorker.detached_10  <=  [0#TcpRouter_main_addr_0]
        // out:  0#RegisterConnectionWorker.detached_10  =>  [0#TcpRouter_api_addr_1]
        // TODO use Context::send_and_receive() ?
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                Address::random_tagged("RegisterConnectionWorker.detached"),
                Arc::new(ockam_core::AllowAll),
                Arc::new(ockam_core::AllowAll),
            ),
            vec![],
        );
        let mut child_ctx = self.ctx.new_detached_with_mailboxes(mailboxes).await?;

        child_ctx
            .send(
                self.api_addr.clone(),
                TcpRouterRequest::Register { accepts, self_addr },
            )
            .await?;
        let response = child_ctx
            .receive::<TcpRouterResponse>()
            .await?
            .take()
            .body();

        if let TcpRouterResponse::Register(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Unregister the connection worker for the given `Address`
    pub async fn unregister(&self, self_addr: Address) -> Result<()> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                TcpRouterRequest::Unregister { self_addr },
            )
            .await?;

        if let TcpRouterResponse::Unregister(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Resolve the given peer to a [`SocketAddr`](std::net::SocketAddr)
    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let hostnames;

        // Try to parse as SocketAddr
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            hostnames = vec![];
        }
        // Try to resolve hostname
        else if let Ok(mut iter) = peer_str.to_socket_addrs() {
            // FIXME: We only take ipv4 for now
            if let Some(p) = iter.find(|x| x.is_ipv4()) {
                peer_addr = p;
            } else {
                return Err(TransportError::InvalidAddress.into());
            }

            hostnames = vec![peer_str];
        }
        // Nothing worked, return an error
        else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok((peer_addr, hostnames))
    }
}

impl TcpRouterHandle {
    /// Bind an incoming portal inlet connection listener for this router
    pub async fn bind_inlet(
        &self,
        outlet_listener_route: impl Into<Route>,
        addr: impl Into<SocketAddr>,
        access_control: Arc<dyn AccessControl>,
    ) -> Result<(Address, SocketAddr)> {
        let socket_addr = addr.into();
        TcpInletListenProcessor::start(
            &self.ctx,
            outlet_listener_route.into(),
            socket_addr,
            access_control,
            // self.main_addr.clone(),
        )
        .await
    }

    /// Stop the inlet's [`TcpInletListenProcessor`]
    pub async fn stop_inlet(&self, addr: impl Into<Address>) -> Result<()> {
        let addr = addr.into();
        debug!(%addr, "stopping inlet");
        self.ctx.stop_processor(addr).await?;
        Ok(())
    }

    /// Stop the inlet's [`TcpPortalWorker`]
    pub async fn stop_outlet(&self, addr: impl Into<Address>) -> Result<()> {
        self.ctx.stop_worker(addr).await?;
        Ok(())
    }
}
