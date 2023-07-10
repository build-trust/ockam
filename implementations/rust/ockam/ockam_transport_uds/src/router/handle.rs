use std::os::unix::net::SocketAddr;

use ockam_core::{
    async_trait, compat::sync::Arc, Address, AsyncTryClone, DenyAll, Mailbox, Mailboxes, Result,
};
use ockam_node::Context;
use ockam_transport_core::TransportError;

use crate::{
    address_from_socket_addr, parse_socket_addr,
    workers::{UdsListenProcessor, WorkerPair},
    UDS,
};

use super::{UdsRouterRequest, UdsRouterResponse};

/// A handle to connect to a [`UdsRouter`](crate::router::UdsRouter)
///
/// Dropping this handle is harmless
pub(crate) struct UdsRouterHandle {
    ctx: Context,
    main_addr: Address,
    api_addr: Address,
}

#[async_trait]
impl AsyncTryClone for UdsRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                Address::random_tagged("UdsRouterHandle.async_try_clone.detached"),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
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

impl UdsRouterHandle {
    /// Create a new [`UdsRouterHandle`] with the given address
    pub(crate) fn new(ctx: Context, main_addr: Address, api_addr: Address) -> Self {
        UdsRouterHandle {
            ctx,
            main_addr,
            api_addr,
        }
    }

    /// Return a reference to the router handle's [`Context`]
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Return a reference to the router handle's [`Main Address`](ockam_core::Address)
    pub(crate) fn main_addr(&self) -> &Address {
        &self.main_addr
    }
}

impl UdsRouterHandle {
    /// Bind an incoming connection listener for this router
    pub async fn bind(&self, addr: impl Into<SocketAddr>) -> Result<SocketAddr> {
        let socket_addr = addr.into();
        UdsListenProcessor::start(&self.ctx, self.async_try_clone().await?, socket_addr).await
    }

    /// Establish an outgoing UDS connection on an existing transport
    pub async fn connect<S: AsRef<str>>(&self, peer: S) -> Result<Address> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                UdsRouterRequest::Connect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        if let UdsRouterResponse::Connect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Disconnect an outgoing UDS connection on an existing transport
    pub async fn disconnect<S: AsRef<str>>(&self, peer: S) -> Result<()> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                UdsRouterRequest::Disconnect {
                    peer: peer.as_ref().to_string(),
                },
            )
            .await?;

        if let UdsRouterResponse::Disconnect(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let uds_address: Address = address_from_socket_addr(pair.peer())?;

        let mut accepts = vec![uds_address];
        accepts.extend(
            pair.paths()
                .iter()
                .map(|x| Address::from_string(format!("{UDS}#{x}"))),
        );
        let self_addr = pair.tx_addr();

        let response: UdsRouterResponse = self
            .ctx()
            .send_and_receive(
                self.api_addr.clone(),
                UdsRouterRequest::Register { accepts, self_addr },
            )
            .await?;

        if let UdsRouterResponse::Register(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Unregister the connection worker for the given [`Address`]
    pub async fn unregister(&self, self_addr: Address) -> Result<()> {
        let response = self
            .ctx
            .send_and_receive(
                self.api_addr.clone(),
                UdsRouterRequest::Unregister { self_addr },
            )
            .await?;
        if let UdsRouterResponse::Unregister(res) = response {
            res
        } else {
            Err(TransportError::InvalidRouterResponseType.into())
        }
    }

    /// Resolve the given peer to a [`SocketAddr`](std::os::unix::net::SocketAddr)
    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let pathnames;

        // Then continue working on resolve_route, so that the UdsRouter can have a complete worker definition which requires `handle_message`
        if let Ok(p) = parse_socket_addr(peer_str) {
            peer_addr = p;
            pathnames = vec![];
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok((peer_addr, pathnames))
    }
}
