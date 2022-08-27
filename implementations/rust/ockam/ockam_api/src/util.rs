use crate::config::lookup::{ConfigLookup, InternetAddress, LookupMeta};
use anyhow::anyhow;
use core::str::FromStr;
use ockam::{Address, TCP};
use ockam_core::{Route, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Node, Project, Service, Space, Tcp};
use ockam_multiaddr::{MultiAddr, Protocol};
use std::net::{SocketAddrV4, SocketAddrV6};

/// Go through a multiaddr and remove all instances of
/// `/node/<whatever>` out of it and replaces it with a fully
/// qualified address to the target
pub fn clean_multiaddr(
    input: &MultiAddr,
    lookup: &ConfigLookup,
) -> Option<(MultiAddr, LookupMeta)> {
    let mut new_ma = MultiAddr::default();
    let mut lookup_meta = LookupMeta::default();

    let it = input.iter().peekable();
    for p in it {
        match p.code() {
            Node::CODE => {
                let alias = p.cast::<Node>()?;
                let addr = lookup
                    .get_node(&alias)
                    .expect("provided invalid substitution route");

                match addr {
                    InternetAddress::Dns(dns, _) => new_ma.push_back(DnsAddr::new(dns)).ok()?,
                    InternetAddress::V4(v4) => new_ma.push_back(Ip4(*v4.ip())).ok()?,
                    InternetAddress::V6(v6) => new_ma.push_back(Ip6(*v6.ip())).ok()?,
                }

                new_ma.push_back(Tcp(addr.port())).ok()?;
            }
            Project::CODE => {
                // Parse project name from the MultiAddr.
                let alias = p.cast::<Project>()?;
                // Store it in the lookup meta, so we can later
                // retrieve it from either the config or the cloud.
                lookup_meta.project.push_back(alias.to_string());
                // No substitution done here. It will be done later by `clean_projects_multiaddr`.
                new_ma.push_back_value(&p).ok()?
            }
            Space::CODE => panic!("/space/ substitutions are not supported yet!"),
            _ => new_ma.push_back_value(&p).ok()?,
        }
    }

    Some((new_ma, lookup_meta))
}

/// Try to convert a multi-address to an Ockam route.
pub fn multiaddr_to_route(ma: &MultiAddr) -> Option<Route> {
    let mut rb = Route::new();
    let mut it = ma.iter().peekable();
    while let Some(p) = it.next() {
        match p.code() {
            Ip4::CODE => {
                let ip4 = p.cast::<Ip4>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV4::new(*ip4, *tcp).to_string());
                rb = rb.append(add)
            }
            Ip6::CODE => {
                let ip6 = p.cast::<Ip6>()?;
                let tcp = it.next()?.cast::<Tcp>()?;
                let add = Address::new(TCP, SocketAddrV6::new(*ip6, *tcp, 0, 0).to_string());
                rb = rb.append(add)
            }
            DnsAddr::CODE => {
                let host = p.cast::<DnsAddr>()?;
                if let Some(p) = it.peek() {
                    if p.code() == Tcp::CODE {
                        let tcp = p.cast::<Tcp>()?;
                        rb = rb.append(Address::new(TCP, format!("{}:{}", &*host, *tcp)));
                        let _ = it.next();
                        continue;
                    }
                }
                rb = rb.append(Address::new(TCP, &*host))
            }
            Service::CODE => {
                let local = p.cast::<Service>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }

            // If your code crashes here then the front-end CLI isn't
            // properly calling `clean_multiaddr` before passing it to
            // the backend
            Node::CODE => unreachable!(),

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(rb.into())
}

/// Try to convert a multiaddr to an Ockam Address
pub fn multiaddr_to_addr(ma: &MultiAddr) -> Option<Address> {
    let mut it = ma.iter().peekable();
    let p = it.next()?;
    match p.code() {
        DnsAddr::CODE => {
            let host = p.cast::<DnsAddr>()?;
            if let Some(p) = it.peek() {
                if p.code() == Tcp::CODE {
                    let tcp = p.cast::<Tcp>()?;
                    return Some(Address::new(TCP, format!("{}:{}", &*host, *tcp)));
                }
            }
            None
        }
        Service::CODE => {
            let local = p.cast::<Service>()?;
            Some(Address::new(LOCAL, &*local))
        }
        _ => None,
    }
}

/// Try to convert an Ockam Route into a MultiAddr.
pub fn route_to_multiaddr(r: &Route) -> Option<MultiAddr> {
    let mut ma = MultiAddr::default();
    for a in r.iter() {
        match a.transport_type() {
            TCP => {
                if let Ok(sa) = SocketAddrV4::from_str(a.address()) {
                    ma.push_back(Ip4::new(*sa.ip())).ok()?;
                    ma.push_back(Tcp::new(sa.port())).ok()?
                } else if let Ok(sa) = SocketAddrV6::from_str(a.address()) {
                    ma.push_back(Ip6::new(*sa.ip())).ok()?;
                    ma.push_back(Tcp::new(sa.port())).ok()?
                } else if let Some((host, port)) = a.address().split_once(':') {
                    ma.push_back(DnsAddr::new(host)).ok()?;
                    ma.push_back(Tcp::new(u16::from_str(port).ok()?)).ok()?
                } else {
                    ma.push_back(DnsAddr::new(a.address())).ok()?
                }
            }
            LOCAL => ma.push_back(Service::new(a.address())).ok()?,
            other => {
                error!(target: "ockam_api", transport = %other, "unsupported transport type");
                return None;
            }
        }
    }
    Some(ma)
}

/// Tells whether the input MultiAddr references a local node or a remote node.
///
/// This should be called before cleaning the MultiAddr.
pub fn is_local_node(ma: &MultiAddr) -> anyhow::Result<bool> {
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
                    .map(|dnsaddr| (&*dnsaddr).eq("localhost"))
                    .ok_or_else(|| anyhow!("Invalid \"dnsaddr\" value"))?;
            }
            // A "/ip4" will be local if it matches the loopback address
            Ip4::CODE => {
                at_rust_node = p
                    .cast::<Ip4>()
                    .map(|ip4| ip4.is_loopback())
                    .ok_or_else(|| anyhow!("Invalid \"ip4\" value"))?;
            }
            // A "/ip6" will be local if it matches the loopback address
            Ip6::CODE => {
                at_rust_node = p
                    .cast::<Ip6>()
                    .map(|ip6| ip6.is_loopback())
                    .ok_or_else(|| anyhow!("Invalid \"ip6\" value"))?;
            }
            // A MultiAddr starting with "/service" could reference both local and remote nodes.
            _ => {
                return Err(anyhow!("Invalid address, protocol not supported"));
            }
        }
        Ok(at_rust_node)
    } else {
        Err(anyhow!("Invalid address"))
    }
}

#[test]
fn clean_multiaddr_simple() {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    let addr: MultiAddr = "/node/hub/service/echoer".parse().unwrap();

    let lookup = {
        let mut map = ConfigLookup::new();
        map.set_node(
            "hub",
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 666)).into(),
        );
        map
    };

    let (new_addr, _) = clean_multiaddr(&addr, &lookup).unwrap();
    assert_ne!(addr, new_addr); // Make sure the address changed

    let new_route = multiaddr_to_route(&new_addr).unwrap();
    println!("{:#?}", new_route);
}
