use std::sync::Arc;
use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionBuilder, Instantiator};
use crate::{multiaddr_to_route, route_to_multiaddr};

use crate::nodes::NodeManager;
use ockam_core::{async_trait, Error, Route};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

/// Creates the tcp connection.
pub(crate) struct PlainTcpInstantiator {}

impl PlainTcpInstantiator {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Instantiator for PlainTcpInstantiator {
    fn matches(&self) -> Vec<Match> {
        vec![
            // matches any tcp address followed by a tcp protocol
            Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]),
            Tcp::CODE.into(),
        ]
    }

    async fn instantiate(
        &self,
        _ctx: Arc<Context>,
        node_manager: &NodeManager,
        _transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, Error> {
        let (before, tcp_piece, after) = extracted;

        let mut tcp = multiaddr_to_route(&tcp_piece, &node_manager.tcp_transport)
            .await
            .ok_or_else(|| {
                ApiError::core(format!(
                    "Couldn't convert MultiAddr to route: tcp_piece={tcp_piece}"
                ))
            })?;

        let multiaddr = route_to_multiaddr(&tcp.route).ok_or_else(|| {
            ApiError::core(format!(
                "Couldn't convert route to MultiAddr: tcp_route={}",
                &tcp.route
            ))
        })?;

        let current_multiaddr = ConnectionBuilder::combine(before, multiaddr, after)?;

        // since we only pass the piece regarding tcp
        // tcp_connection should exist
        let tcp_connection = tcp
            .tcp_connection
            .take()
            .ok_or_else(|| ApiError::core("TCP connection should be set"))?;

        Ok(Changes {
            current_multiaddr,
            flow_control_id: tcp.flow_control_id,
            secure_channel_encryptors: vec![],
            tcp_connection: Some(tcp_connection),
        })
    }
}
