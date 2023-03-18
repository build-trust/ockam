use crate::{
    rendezvous_service::{RendezvousRequest, RendezvousResponse},
    UDP,
};
use ockam_core::{async_trait, Address, Result, Route, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::{collections::BTreeMap, net::SocketAddr};
use tracing::{debug, error, trace};

/// Worker for the UDP NAT Hole Punching Rendezvous service
///
/// Maintains an internal map for remote nodes and the public IP address
/// from which they send UDP datagrams.
///
/// Remote nodes can send requests to update and query the map.
pub struct RendezvousWorker {
    map: BTreeMap<String, SocketAddr>,
}

impl Default for RendezvousWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl RendezvousWorker {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Get address of next UDP hop
    fn recent_udp_hop(route: &Route) -> Option<Address> {
        route.iter().find(|x| x.transport_type() == UDP).cloned()
    }

    // Handle Update request
    fn handle_update(&mut self, node_name: &str, return_route: &Route) -> Result<()> {
        match Self::recent_udp_hop(return_route) {
            Some(hop) => match hop.address().parse::<SocketAddr>() {
                Ok(addr) => {
                    self.map.insert(node_name.to_owned(), addr);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to parse '{:?}' into a socket address: {:?}", hop, e);
                    Err(TransportError::InvalidAddress.into())
                }
            },
            None => {
                // This can happen, for example, if a client erroneously sends a request via TCP not UDP
                error!("No UDP hop in message's return route: {:?}", return_route);
                Err(TransportError::InvalidAddress.into())
            }
        }
    }

    // Handle Query request
    fn handle_query(&self, node_name: &String) -> Result<SocketAddr> {
        match self.map.get(node_name) {
            Some(addr) => Ok(*addr),
            None => Err(TransportError::PeerNotFound.into()),
        }
    }
}

#[async_trait]
impl Worker for RendezvousWorker {
    type Message = RendezvousRequest;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        debug!("Received message: {:?}", msg);
        let return_route = msg.return_route();
        match msg.as_body() {
            RendezvousRequest::Update { node_name } => {
                let res = self.handle_update(node_name, &return_route);
                ctx.send(return_route, RendezvousResponse::Update(res))
                    .await?;
            }
            RendezvousRequest::Query { node_name } => {
                let res = self.handle_query(node_name);
                ctx.send(return_route, RendezvousResponse::Query(res))
                    .await?;
            }
        }
        trace!("Map: {:?}", self.map);
        Ok(())
    }
}
