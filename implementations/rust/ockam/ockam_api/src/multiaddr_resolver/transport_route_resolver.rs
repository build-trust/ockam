use std::net::{SocketAddrV4, SocketAddrV6};

use crate::multiaddr_resolver::{invalid_multiaddr_error, multiple_transport_hops_error};
use ockam::tcp::TCP;
use ockam::udp::UDP;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Address, Error, Result, Route, TransportType, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Secure, Service, Tcp, Udp, Worker};
use ockam_multiaddr::{MultiAddr, ProtoIter, ProtoValue, Protocol};

#[derive(Default, Debug, Clone)]
pub struct TransportRouteResolver {
    allow_tcp: bool,
    allow_udp: bool,
}

impl TransportRouteResolver {
    pub fn new(allow_tcp: bool, allow_udp: bool) -> Self {
        Self {
            allow_tcp,
            allow_udp,
        }
    }

    pub fn allow_tcp(&mut self) -> &mut Self {
        self.allow_tcp = true;
        self
    }

    pub fn allow_udp(&mut self) -> &mut Self {
        self.allow_udp = true;
        self
    }
}

impl TransportRouteResolver {
    /// Resolve all the multiaddresses which represent transport addresses
    /// For example /tcp/127.0.0.1/port/4000 is transformed to the Address (TCP, "127.0.0.1:4000")
    /// The creation of a TCP worker and the substitution of that transport address to a worker address
    /// is done later with `context.resolve_transport_route(route)`
    pub fn resolve(&self, ma: &MultiAddr) -> Result<Route> {
        let mut route = Route::new();
        let mut it = ma.iter();

        // Only one transport hop is allowed
        let mut transport_hop_resolved = false;

        while let Some(p) = it.next() {
            match p.code() {
                Ip4::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }
                    let ip4 = p.cast::<Ip4>().ok_or_else(|| invalid_multiaddr_error(ma))?;
                    let (transport_type, port) = self.parse_port_it(ma, &mut it)?;
                    let socket_addr = SocketAddrV4::new(*ip4, port);
                    route = route.append(Address::new_with_string(
                        transport_type,
                        socket_addr.to_string(),
                    ));
                    transport_hop_resolved = true;
                }
                Ip6::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }
                    let ip6 = p.cast::<Ip6>().ok_or_else(|| invalid_multiaddr_error(ma))?;
                    let (transport_type, port) = self.parse_port_it(ma, &mut it)?;
                    let socket_addr = SocketAddrV6::new(*ip6, port, 0, 0);
                    route = route.append(Address::new_with_string(
                        transport_type,
                        socket_addr.to_string(),
                    ));
                    transport_hop_resolved = true;
                }
                DnsAddr::CODE => {
                    if transport_hop_resolved {
                        return Err(multiple_transport_hops_error(ma));
                    }
                    let host = p
                        .cast::<DnsAddr>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    let (transport_type, port) = self.parse_port_it(ma, &mut it)?;
                    let addr = format!("{}:{}", &*host, port);
                    route = route.append(Address::new_with_string(transport_type, addr));
                    transport_hop_resolved = true;
                }
                Worker::CODE => {
                    let local = p
                        .cast::<Worker>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    route = route.append(Address::new_with_string(LOCAL, &*local))
                }
                Service::CODE => {
                    let local = p
                        .cast::<Service>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    route = route.append(Address::new_with_string(LOCAL, &*local))
                }
                Secure::CODE => {
                    let local = p
                        .cast::<Secure>()
                        .ok_or_else(|| invalid_multiaddr_error(ma))?;
                    route = route.append(Address::new_with_string(LOCAL, &*local))
                }
                _ => {
                    return Err(Error::new(
                        Origin::Api,
                        Kind::Misuse,
                        format!("Unsupported multiaddr protocol: {}", ma),
                    ));
                }
            }
        }

        Ok(route.into())
    }
    /// If the input MultiAddr is "/dnsaddr/localhost/tcp/4000/service/api",
    /// then this will return string format of the SocketAddr: "127.0.0.1:4000".
    pub fn socket_address(&self, ma: &MultiAddr) -> Result<String> {
        let mut it = ma.iter();

        let first = it.next().ok_or_else(|| invalid_multiaddr_error(ma))?;
        let second = it.next().ok_or_else(|| invalid_multiaddr_error(ma))?;

        match first.code() {
            Ip4::CODE => {
                let ip4 = first
                    .cast::<Ip4>()
                    .ok_or_else(|| invalid_multiaddr_error(ma))?;
                let (_transport_type, port) = self.parse_port(ma, &second)?;
                Ok(SocketAddrV4::new(*ip4, port).to_string())
            }
            Ip6::CODE => {
                let ip6 = first
                    .cast::<Ip6>()
                    .ok_or_else(|| invalid_multiaddr_error(ma))?;
                let (_transport_type, port) = self.parse_port(ma, &second)?;
                Ok(SocketAddrV6::new(*ip6, port, 0, 0).to_string())
            }
            DnsAddr::CODE => {
                let host = first
                    .cast::<DnsAddr>()
                    .ok_or_else(|| invalid_multiaddr_error(ma))?;
                let (_transport_type, port) = self.parse_port(ma, &second)?;
                Ok(format!("{}:{}", &*host, port))
            }
            _ => Err(invalid_multiaddr_error(ma)),
        }
    }

    fn parse_port_it(&self, ma: &MultiAddr, it: &mut ProtoIter) -> Result<(TransportType, u16)> {
        let next = it.next().ok_or_else(|| invalid_multiaddr_error(ma))?;

        self.parse_port(ma, &next)
    }

    fn parse_port(&self, ma: &MultiAddr, next: &ProtoValue) -> Result<(TransportType, u16)> {
        if let Some(port) = next.cast::<Tcp>() {
            if !self.allow_tcp {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Unsupported,
                    format!("TCP hops are not allowed. Multiaddr={}", ma),
                ));
            }

            return Ok((TCP, port.0));
        }

        if let Some(port) = next.cast::<Udp>() {
            if !self.allow_udp {
                return Err(Error::new(
                    Origin::Api,
                    Kind::Unsupported,
                    format!("UDP hops are not allowed. Multiaddr={}", ma),
                ));
            }

            return Ok((UDP, port.0));
        }

        // Should not happen
        Err(invalid_multiaddr_error(ma))
    }
}
