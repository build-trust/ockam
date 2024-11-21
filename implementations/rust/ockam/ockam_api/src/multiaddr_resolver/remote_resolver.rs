use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use crate::multiaddr_resolver::{invalid_multiaddr_error, multiple_transport_hops_error};
use ockam::tcp::{TcpConnection, TcpConnectionOptions, TcpTransport};
use ockam::udp::{UdpBind, UdpBindArguments, UdpBindOptions, UdpTransport};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Error, Result, Route, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Secure, Service, Tcp, Udp, Worker};
use ockam_multiaddr::{MultiAddr, ProtoIter, Protocol};

pub enum RemoteMultiaddrResolverConnection {
    Tcp(TcpConnection),
    Udp(UdpBind),
}

impl RemoteMultiaddrResolverConnection {
    fn flow_control_id(&self) -> &FlowControlId {
        match self {
            RemoteMultiaddrResolverConnection::Tcp(c) => c.flow_control_id(),
            RemoteMultiaddrResolverConnection::Udp(b) => b.flow_control_id(),
        }
    }

    fn sender_address(&self) -> &Address {
        match self {
            RemoteMultiaddrResolverConnection::Tcp(t) => t.sender_address(),
            RemoteMultiaddrResolverConnection::Udp(b) => b.sender_address(),
        }
    }
}

pub struct RemoteMultiaddrResolverResult {
    pub flow_control_id: Option<FlowControlId>,
    pub route: Route,
    pub connection: Option<RemoteMultiaddrResolverConnection>,
}

#[derive(Default, Clone, Debug)]
pub struct RemoteMultiaddrResolver {
    tcp: Option<TcpTransport>,
    udp: Option<UdpTransport>,
    udp_bind_address: Option<SocketAddr>,
}

impl RemoteMultiaddrResolver {
    pub fn new(tcp: Option<TcpTransport>, udp: Option<UdpTransport>) -> Self {
        Self {
            tcp,
            udp,
            udp_bind_address: None,
        }
    }

    pub fn with_tcp(&mut self, tcp: TcpTransport) -> &mut Self {
        self.tcp = Some(tcp);
        self
    }

    pub fn with_udp(&mut self, udp: UdpTransport, bind_address: Option<SocketAddr>) -> &mut Self {
        self.udp = Some(udp);
        self.udp_bind_address = bind_address;
        self
    }
}

fn unsupported_protocol_error(ma: &MultiAddr) -> Error {
    Error::new(
        Origin::Api,
        Kind::Unsupported,
        format!("Unsupported multiaddr protocol: {}", ma),
    )
}

impl RemoteMultiaddrResolver {
    pub async fn resolve(&self, ma: &MultiAddr) -> Result<RemoteMultiaddrResolverResult> {
        let mut rb = Route::new();
        let mut it = ma.iter();

        let mut flow_control_id = None;
        let mut connection_res = None;

        // Only one transport hop is allowed
        let mut transport_hop_resolved = false;

        while let Some(p) = it.next() {
            let peer = match p.code() {
                Ip4::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }

                    let ip4 = p.cast::<Ip4>().ok_or_else(|| invalid_multiaddr_error(ma))?;

                    (*ip4).to_string()
                }
                Ip6::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }

                    let ip6 = p.cast::<Ip6>().ok_or_else(|| invalid_multiaddr_error(ma))?;

                    (*ip6).to_string()
                }
                DnsAddr::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }

                    let host = p
                        .cast::<DnsAddr>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;

                    (*host).to_string()
                }
                Worker::CODE => {
                    let local = p
                        .cast::<Worker>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local));
                    continue;
                }
                Service::CODE => {
                    let local = p
                        .cast::<Service>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local));
                    continue;
                }
                Secure::CODE => {
                    let local = p
                        .cast::<Secure>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    rb = rb.append(Address::new_with_string(LOCAL, &*local));
                    continue;
                }
                _ => {
                    return Err(unsupported_protocol_error(ma));
                }
            };

            let connection = self.connect(ma, &mut it, &peer).await?;
            transport_hop_resolved = true;
            flow_control_id = Some(connection.flow_control_id().clone());
            rb = rb.append(connection.sender_address().clone());
            connection_res = Some(connection);
        }

        Ok(RemoteMultiaddrResolverResult {
            flow_control_id,
            connection: connection_res,
            route: rb.into(),
        })
    }

    async fn connect_tcp(
        &self,
        tcp: &TcpTransport,
        ma: &MultiAddr,
        peer: String,
    ) -> Result<TcpConnection> {
        tcp.connect(peer, TcpConnectionOptions::new())
            .await
            .map_err(|err| {
                Error::new(
                    Origin::Api,
                    Kind::Io,
                    format!(
                        "Couldn't make TCP connection while resolving multiaddr: {}. Err: {}",
                        ma, err
                    ),
                )
            })
    }

    async fn connect_udp(&self, udp: &UdpTransport, ma: &MultiAddr, peer: &str) -> Result<UdpBind> {
        let bind_address = self
            .udp_bind_address
            .unwrap_or_else(|| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)));

        let arguments = UdpBindArguments::new()
            .with_bind_socket_address(bind_address)
            .with_peer_address(peer)
            .await?;

        udp.bind(arguments, UdpBindOptions::new())
            .await
            .map_err(|err| {
                Error::new(
                    Origin::Api,
                    Kind::Io,
                    format!(
                        "Couldn't make UDP connection while resolving multiaddr: {}. Err: {}",
                        ma, err
                    ),
                )
            })
    }

    async fn connect(
        &self,
        ma: &MultiAddr,
        it: &mut ProtoIter<'_>,
        peer: &str,
    ) -> Result<RemoteMultiaddrResolverConnection> {
        let next = it.next().ok_or_else(|| invalid_multiaddr_error(ma))?;

        if let Some(port) = next.cast::<Tcp>() {
            let tcp = self.tcp.as_ref().ok_or_else(|| {
                Error::new(
                    Origin::Api,
                    Kind::Unsupported,
                    format!("TCP hops are not allowed. Multiaddr={}", ma),
                )
            })?;

            let peer = format!("{}:{}", peer, *port);
            let connection = self.connect_tcp(tcp, ma, peer).await?;

            return Ok(RemoteMultiaddrResolverConnection::Tcp(connection));
        }

        if let Some(port) = next.cast::<Udp>() {
            let udp = self.udp.as_ref().ok_or_else(|| {
                Error::new(
                    Origin::Api,
                    Kind::Unsupported,
                    format!("UDP hops are not allowed. Multiaddr={}", ma),
                )
            })?;

            let peer = format!("{}:{}", peer, *port);
            let connection = self.connect_udp(udp, ma, &peer).await?;

            return Ok(RemoteMultiaddrResolverConnection::Udp(connection));
        }

        Err(unsupported_protocol_error(ma))
    }
}
