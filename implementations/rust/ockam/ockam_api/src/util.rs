use miette::miette;

use ockam_core::Result;
use ockam_multiaddr::proto::{
    DnsAddr, Ip4, Ip6, Node, Project, Secure, Service, Space, Tcp, Worker,
};
use ockam_multiaddr::{Code, MultiAddr, Protocol};

use crate::error::ApiError;

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
