use std::os::unix::net::SocketAddr;

use ockam_core::{Address, Result, TryClone};
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
#[derive(TryClone)]
#[try_clone(crate = "ockam_core")]
pub(crate) struct UdsRouterHandle {
    ctx: Context,
    main_addr: Address,
    api_addr: Address,
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
    pub fn bind(&self, addr: impl Into<SocketAddr>) -> Result<SocketAddr> {
        let socket_addr = addr.into();
        UdsListenProcessor::start(&self.ctx, self.try_clone()?, socket_addr)
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
            Err(TransportError::InvalidRouterResponseType)?
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
            Err(TransportError::InvalidRouterResponseType)?
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
            Err(TransportError::InvalidRouterResponseType)?
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
            Err(TransportError::InvalidRouterResponseType)?
        }
    }

    /// Resolve the given peer to a [`SocketAddr`](std::os::unix::net::SocketAddr)
    // TODO: Remove in favor of `ockam_node::compat::asynchronous::resolve_peer`
    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(SocketAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let pathnames;

        // Then continue working on resolve_route, so that the UdsRouter can have a complete worker definition which requires `handle_message`
        if let Ok(p) = parse_socket_addr(peer_str.clone()) {
            peer_addr = p;
            pathnames = vec![];
        } else {
            return Err(TransportError::InvalidAddress(peer_str))?;
        }

        Ok((peer_addr, pathnames))
    }
}
