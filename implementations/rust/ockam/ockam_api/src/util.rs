use crate::cli_state::CliState;
use crate::config::lookup::{InternetAddress, LookupMeta};
use crate::error::ApiError;
use anyhow::anyhow;
use ockam::{Address, Error, Result};
use ockam_core::{Route, LOCAL};
use ockam_multiaddr::proto::{DnsAddr, Ip4, Ip6, Node, Project, Secure, Service, Space, Tcp, OTCP};
use ockam_multiaddr::{MultiAddr, Protocol};

/// Go through a multiaddr and remove all instances of
/// `/node/<whatever>` out of it and replaces it with a fully
/// qualified address to the target
pub fn clean_multiaddr(input: &MultiAddr, cli_state: &CliState) -> Result<(MultiAddr, LookupMeta)> {
    let mut new_ma = MultiAddr::default();
    let mut lookup_meta = LookupMeta::default();

    let it = input.iter().peekable();
    for p in it {
        match p.code() {
            Node::CODE => {
                let alias = p.cast::<Node>().expect("Failed to parse node name");
                let node_setup = cli_state.nodes.get(&alias)?.setup()?;
                let addr = &node_setup.default_tcp_listener()?.addr;
                match addr {
                    InternetAddress::Dns(dns, _) => new_ma.push_back(DnsAddr::new(dns))?,
                    InternetAddress::V4(v4) => new_ma.push_back(Ip4(*v4.ip()))?,
                    InternetAddress::V6(v6) => new_ma.push_back(Ip6(*v6.ip()))?,
                }
                new_ma.push_back(Tcp(addr.port()))?;
            }
            Project::CODE => {
                // Parse project name from the MultiAddr.
                let alias = p.cast::<Project>().expect("Failed to parse project name");
                // Store it in the lookup meta, so we can later
                // retrieve it from either the config or the cloud.
                lookup_meta.project.push_back(alias.to_string());
                // No substitution done here. It will be done later by `clean_projects_multiaddr`.
                new_ma.push_back_value(&p)?
            }
            Space::CODE => panic!("/space/ substitutions are not supported yet!"),
            _ => new_ma.push_back_value(&p)?,
        }
    }

    Ok((new_ma, lookup_meta))
}

/// Try to convert a multi-address to an Ockam route.
pub fn multiaddr_to_route(ma: &MultiAddr) -> Option<Route> {
    let mut rb = Route::new();
    for p in ma.iter() {
        match p.code() {
            // Only hops that are directly translated to existing workers are allowed here
            Service::CODE => {
                let local = p.cast::<Service>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            Secure::CODE => {
                let local = p.cast::<Secure>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }
            OTCP::CODE => {
                let local = p.cast::<OTCP>()?;
                rb = rb.append(Address::new(LOCAL, &*local))
            }

            // If your code crashes here then the front-end CLI isn't
            // properly calling `clean_multiaddr` before passing it to
            // the backend
            Node::CODE => unreachable!(),
            Ip4::CODE | Ip6::CODE | DnsAddr::CODE => unreachable!(),

            other => {
                error!(target: "ockam_api", code = %other, "unsupported protocol");
                return None;
            }
        }
    }
    Some(rb.into())
}

pub fn try_multiaddr_to_route(ma: &MultiAddr) -> Result<Route, Error> {
    multiaddr_to_route(ma)
        .ok_or_else(|| ApiError::message(format!("could not convert {ma} to route")))
}

/// Try to convert a multiaddr to an Ockam Address
pub fn multiaddr_to_addr(ma: &MultiAddr) -> Option<Address> {
    let mut it = ma.iter().peekable();
    let p = it.next()?;
    match p.code() {
        Service::CODE => {
            let local = p.cast::<Service>()?;
            Some(Address::new(LOCAL, &*local))
        }
        OTCP::CODE => {
            let local = p.cast::<OTCP>()?;
            Some(Address::new(LOCAL, &*local))
        }
        _ => None,
    }
}

pub fn try_multiaddr_to_addr(ma: &MultiAddr) -> Result<Address, Error> {
    multiaddr_to_addr(ma)
        .ok_or_else(|| ApiError::message(format!("could not convert {ma} to address")))
}

/// Try to convert an Ockam Route into a MultiAddr.
pub fn route_to_multiaddr(r: &Route) -> Option<MultiAddr> {
    let mut ma = MultiAddr::default();
    for a in r.iter() {
        ma.try_extend(&try_address_to_multiaddr(a).ok()?).ok()?
    }
    Some(ma)
}

/// Try to convert an Ockam Address to a MultiAddr.
pub fn try_address_to_multiaddr(a: &Address) -> Result<MultiAddr, Error> {
    let mut ma = MultiAddr::default();
    match a.transport_type() {
        LOCAL => ma.push_back(Service::new(a.address()))?,
        other => {
            error!(target: "ockam_api", transport = %other, "unsupported transport type");
            return Err(ApiError::message(format!(
                "unknown transport type: {other}"
            )));
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
                    .map(|dnsaddr| (*dnsaddr).eq("localhost"))
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
    let addr: MultiAddr = "/project/hub/service/echoer".parse().unwrap();
    let (_new_addr, lookup_meta) = clean_multiaddr(&addr, &CliState::new().unwrap()).unwrap();
    assert!(lookup_meta.project.contains(&"hub".to_string()));
}
