use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionBuilder, Instantiator};
use crate::{RemoteMultiaddrResolver, RemoteMultiaddrResolverConnection, ReverseLocalConverter};

use crate::nodes::NodeManager;
use ockam_core::{async_trait, Error, Route};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Udp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;

/// Creates the tcp connection.
pub(crate) struct PlainUdpInstantiator {}

impl PlainUdpInstantiator {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Instantiator for PlainUdpInstantiator {
    fn matches(&self) -> Vec<Match> {
        vec![
            // matches any tcp address followed by a tcp protocol
            Match::any([DnsAddr::CODE, Ip4::CODE, Ip6::CODE]),
            Udp::CODE.into(),
        ]
    }

    async fn instantiate(
        &self,
        _ctx: &Context,
        node_manager: &NodeManager,
        _transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, Error> {
        let (before, udp_piece, after) = extracted;

        let mut udp = RemoteMultiaddrResolver::new(None, node_manager.udp_transport.clone())
            .resolve(&udp_piece)
            .await?;

        let multiaddr = ReverseLocalConverter::convert_route(&udp.route)?;

        let current_multiaddr = ConnectionBuilder::combine(before, multiaddr, after)?;

        // since we only pass the piece regarding udp
        // udp_bind should exist
        let udp_bind = udp
            .connection
            .take()
            .ok_or_else(|| ApiError::core("UDP connection should be set"))?;

        let udp_bind = match udp_bind {
            RemoteMultiaddrResolverConnection::Tcp(_) => {
                return Err(ApiError::core("UDP connection should be set"));
            }
            RemoteMultiaddrResolverConnection::Udp(udp_bind) => udp_bind,
        };

        Ok(Changes {
            current_multiaddr,
            flow_control_id: udp.flow_control_id,
            secure_channel_encryptors: vec![],
            tcp_connection: None,
            udp_bind: Some(udp_bind),
        })
    }
}
