use std::net::{SocketAddrV4, SocketAddrV6};

use miette::miette;

use ockam::tcp::{TcpConnection, TcpConnectionOptions, TcpTransport, TCP};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControlId;
use ockam_core::{Address, Error, Result, Route, TransportType, LOCAL};
use ockam_multiaddr::proto::{
    DnsAddr, Ip4, Ip6, Node, Project, Secure, Service, Space, Tcp, Worker,
};
use ockam_multiaddr::{Code, MultiAddr, Protocol};

use crate::error::ApiError;

/// Try to convert a multi-address to an Ockam route.
pub fn local_multiaddr_to_route(ma: &MultiAddr) -> Result<Route> {
    let mut rb = Route::new();
    for p in ma.iter() {
        match p.code() {
            // Only hops that are directly translated to existing workers are allowed here
            Worker::CODE => {
                let local = p.cast::<Worker>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect worker address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect service address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>().ok_or(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("incorrect secure address {ma})",),
                ))?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }

            Node::CODE => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    "unexpected code: node. clean_multiaddr should have been called",
                ));
            }

            code @ (Ip4::CODE | Ip6::CODE | DnsAddr::CODE) => {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("unexpected code: {code}. The address must be a local address {ma}"),
                ));
            }

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return Err(Error::new(
                    Origin::Api,
                    Kind::Invalid,
                    format!("unsupported protocol {other}"),
                ));
            }
        }
    }

    Ok(rb.into())
}

pub struct MultiAddrToRouteResult {
    pub flow_control_id: Option<FlowControlId>,
    pub route: Route,
    pub tcp_connection: Option<TcpConnection>,
}

pub async fn multiaddr_to_route(
    ma: &MultiAddr,
    tcp: &TcpTransport,
) -> Option<MultiAddrToRouteResult> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();

    let mut flow_control_id = None;
    let mut number_of_tcp_hops = 0;
    let mut tcp_connection = None;

    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let ip4 = p.cast::<Ip4>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV4::new(*ip4, *port);

                let options = TcpConnectionOptions::new();
                flow_control_id = Some(options.flow_control_id().clone());

                let connection = match tcp.connect(socket_addr.to_string(), options).await {
                    Ok(c) => c,
                    Err(error) => {
                        error!(%error, %socket_addr, "Couldn't connect to Ip4 address");
                        return None;
                    }
                };

                number_of_tcp_hops += 1;
                rb = rb.append(connection.sender_address().clone());

                tcp_connection = Some(connection);
            }
            Ip6::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let ip6 = p.cast::<Ip6>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);

                let options = TcpConnectionOptions::new();
                flow_control_id = Some(options.flow_control_id().clone());

                let connection = match tcp.connect(socket_addr.to_string(), options).await {
                    Ok(c) => c,
                    Err(error) => {
                        error!(%error, %socket_addr, "Couldn't connect to Ip6 address");
                        return None;
                    }
                };

                number_of_tcp_hops += 1;
                rb = rb.append(connection.sender_address().clone());

                tcp_connection = Some(connection);
            }
            DnsAddr::CODE => {
                if number_of_tcp_hops >= 1 {
                    return None; // Only 1 TCP hop is allowed
                }

                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let port = p.cast::<Tcp>()?;

                        let options = TcpConnectionOptions::new();
                        flow_control_id = Some(options.flow_control_id().clone());
                        let peer = format!("{}:{}", &*host, *port);

                        let connection = match tcp.connect(&peer, options).await {
                            Ok(c) => c,
                            Err(error) => {
                                error!(%error, %peer, "Couldn't connect to DNS address");
                                return None;
                            }
                        };

                        number_of_tcp_hops += 1;
                        rb = rb.append(connection.sender_address().clone());

                        tcp_connection = Some(connection);

                        let _ = it.next();

                        continue;
                    }
                }
            }
            Worker::CODE => {
                let local = p.cast::<Worker>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }

    Some(MultiAddrToRouteResult {
        flow_control_id,
        tcp_connection,
        route: rb.into(),
    })
}

/// Resolve all the multiaddresses which represent transport addresses
/// For example /tcp/127.0.0.1/port/4000 is transformed to the Address (TCP, "127.0.0.1:4000")
/// The creation of a TCP worker and the substitution of that transport address to a worker address
/// is done later with `context.resolve_transport_route(route)`
pub fn multiaddr_to_transport_route(ma: &MultiAddr) -> Option<Route> {
    let mut route = Route::new();
    let mut it = ma.iter().peekable();

    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                let ip4 = p.cast::<Ip4>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV4::new(*ip4, *port);
                route = route.append(Address::new(TCP, socket_addr.to_string()))
            }
            Ip6::CODE => {
                let ip6 = p.cast::<Ip6>()?;
                let port = it.next()?.cast::<Tcp>()?;
                let socket_addr = SocketAddrV6::new(*ip6, *port, 0, 0);
                route = route.append(Address::new(TransportType::new(1), socket_addr.to_string()))
            }
            DnsAddr::CODE => {
                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let port = p.cast::<Tcp>()?;
                        let addr = format!("{}:{}", &*host, *port);
                        route = route.append(Address::new(TransportType::new(1), addr));
                        let _ = it.next();
                        continue;
                    }
                }
            }
            Worker::CODE => {
                let local = p.cast::<Worker>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                route = route.append(Address::new(LOCAL, &*local))
            }
            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(route.into())
}

/// Try to convert a multiaddr to an Ockam Address
pub fn multiaddr_to_addr(ma: &MultiAddr) -> Option<Address> {
    let mut it = ma.iter().peekable();
    let p = it.next()?;
    match p.code() {
        Worker::CODE => {
            let local = p.cast::<Worker>()?;
            Some(Address::new(LOCAL, &*local))
        }
        Service::CODE => {
            let local = p.cast::<Service>()?;
            Some(Address::new(LOCAL, &*local))
        }
        _ => None,
    }
}

pub fn try_multiaddr_to_addr(ma: &MultiAddr) -> Result<Address, Error> {
    multiaddr_to_addr(ma)
        .ok_or_else(|| ApiError::core(format!("could not convert {ma} to address")))
}

/// Convert an Ockam Route into a MultiAddr.
pub fn route_to_multiaddr(r: &Route) -> Option<MultiAddr> {
    try_route_to_multiaddr(r).ok()
}

/// Try to convert an Ockam Route into a MultiAddr.
pub fn try_route_to_multiaddr(r: &Route) -> Result<MultiAddr, Error> {
    let mut ma = MultiAddr::default();
    for a in r.iter() {
        ma.try_extend(&try_address_to_multiaddr(a)?)?
    }
    Ok(ma)
}

/// Try to convert an Ockam Address to a MultiAddr.
pub fn try_address_to_multiaddr(a: &Address) -> Result<MultiAddr, Error> {
    let mut ma = MultiAddr::default();
    match a.transport_type() {
        LOCAL => ma.push_back(Service::new(a.address()))?,
        other => {
            error!(target: "ockam_api", transport = %other, "unsupported transport type");
            return Err(ApiError::core(format!("unknown transport type: {other}")));
        }
    }
    Ok(ma)
}

/// Try to convert an Ockam Address into a MultiAddr.
pub fn addr_to_multiaddr<T: Into<Address>>(a: T) -> Option<MultiAddr> {
    let r: Route = Route::from(a);
    route_to_multiaddr(&r)
}

/// Tells whether the input MultiAddr references a local node or a remote node.
///
/// This should be called before cleaning the MultiAddr.
pub fn is_local_node(ma: &MultiAddr) -> miette::Result<bool> {
    let at_rust_node;
    if let Some(p) = ma.iter().next() {
        match p.code() {
            // A MultiAddr starting with "/project" will always reference a remote node.
            Project::CODE => {
                at_rust_node = false;
            }
            // A MultiAddr starting with "/node" will always reference a local node.
            Node::CODE => {
                at_rust_node = true;
            }
            // A "/dnsaddr" will be local if it is "localhost"
            DnsAddr::CODE => {
                at_rust_node = p
                    .cast::<DnsAddr>()
                    .map(|dnsaddr| (*dnsaddr).eq("localhost"))
                    .ok_or_else(|| miette!("Invalid \"dnsaddr\" value"))?;
            }
            // A "/ip4" will be local if it matches the loopback address
            Ip4::CODE => {
                at_rust_node = p
                    .cast::<Ip4>()
                    .map(|ip4| ip4.is_loopback())
                    .ok_or_else(|| miette!("Invalid \"ip4\" value"))?;
            }
            // A "/ip6" will be local if it matches the loopback address
            Ip6::CODE => {
                at_rust_node = p
                    .cast::<Ip6>()
                    .map(|ip6| ip6.is_loopback())
                    .ok_or_else(|| miette!("Invalid \"ip6\" value"))?;
            }
            // A MultiAddr starting with "/service" could reference both local and remote nodes.
            _ => {
                return Err(miette!("Invalid address, protocol not supported"));
            }
        }
        Ok(at_rust_node)
    } else {
        Err(miette!("Invalid address"))
    }
}

/// Tells whether the input [`Code`] references a local worker.
pub fn local_worker(code: &Code) -> Result<bool> {
    match *code {
        Node::CODE
        | Space::CODE
        | Project::CODE
        | DnsAddr::CODE
        | Ip4::CODE
        | Ip6::CODE
        | Tcp::CODE
        | Secure::CODE => Ok(false),
        Worker::CODE | Service::CODE => Ok(true),

        _ => Err(ApiError::core(format!("unknown transport type: {code}"))),
    }
}
