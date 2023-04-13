use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionInstanceBuilder, Instantiator};
use crate::{multiaddr_to_route, route_to_multiaddr};

use ockam_core::{async_trait, Error};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::{Match, Protocol};
use ockam_transport_tcp::TcpTransport;

/// Creates the tcp connection.
pub(crate) struct PlainTcpInstantiator {
    tcp_transport: TcpTransport,
}

impl PlainTcpInstantiator {
    pub(crate) fn new(tcp_transport: TcpTransport) -> Self {
        Self { tcp_transport }
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
        builder: &ConnectionInstanceBuilder,
        match_start: usize,
    ) -> Result<Changes, Error> {
        let (before, tcp_piece, after) =
            ConnectionInstanceBuilder::extract(&builder.current_multiaddr, match_start, 2);

        let tcp = multiaddr_to_route(&tcp_piece, &self.tcp_transport)
            .await
            .ok_or_else(|| ApiError::generic("invalid multiaddr"))?;

        let multiaddr =
            route_to_multiaddr(&tcp.route).ok_or_else(|| ApiError::generic("invalid tcp route"))?;

        let current_multiaddr = ConnectionInstanceBuilder::combine(before, multiaddr, after)?;

        Ok(Changes {
            current_multiaddr,
            flow_control_id: tcp.flow_control_id,
            secure_channel_encryptors: vec![],
            //since we only pass the piece regarding tcp
            //we can be sure the next step is the tcp worker
            tcp_worker: Some(tcp.route.next()?.clone()),
        })
    }
}
