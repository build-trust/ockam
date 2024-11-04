use crate::error::ApiError;
use crate::nodes::connection::{Changes, ConnectionBuilder, Instantiator};
use crate::{RemoteMultiaddrResolver, RemoteMultiaddrResolverConnection, ReverseLocalConverter};

use ockam_core::{async_trait, Route};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Tcp};
use ockam_multiaddr::{Match, MultiAddr, Protocol};
use ockam_node::Context;
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
        _context: &Context,
        _transport_route: Route,
        extracted: (MultiAddr, MultiAddr, MultiAddr),
    ) -> Result<Changes, ockam_core::Error> {
        let (before, tcp_piece, after) = extracted;

        let mut tcp = RemoteMultiaddrResolver::default()
            .with_tcp(self.tcp_transport.clone())
            .resolve(&tcp_piece)
            .await?;

        let multiaddr = ReverseLocalConverter::convert_route(&tcp.route)?;

        let current_multiaddr = ConnectionBuilder::combine(before, multiaddr, after)?;

        // since we only pass the piece regarding tcp
        // tcp_connection should exist
        let tcp_connection = tcp
            .connection
            .take()
            .ok_or_else(|| ApiError::core("TCP connection should be set"))?;

        let tcp_connection = match tcp_connection {
            RemoteMultiaddrResolverConnection::Tcp(tcp_connection) => tcp_connection,
            RemoteMultiaddrResolverConnection::Udp(_) => {
                return Err(ApiError::core("TCP connection should be set"));
            }
        };

        Ok(Changes {
            current_multiaddr,
            flow_control_id: tcp.flow_control_id,
            secure_channel_encryptors: vec![],
            tcp_connection: Some(tcp_connection),
            udp_bind: None,
        })
    }
}
