use crate::error::ApiError;
use crate::CliState;
use miette::miette;
use ockam_core::Result;
use ockam_multiaddr::proto::{Node, Project, Service};
use ockam_multiaddr::{MultiAddr, Protocol};
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;

/// Get address value from a string.
///
/// The input string can be either a plain address of a MultiAddr formatted string.
/// Examples: `/node/<name>`, `<name>`
pub fn extract_address_value(input: &str) -> Result<String, ApiError> {
    // we default to the `input` value
    let mut addr = input.to_string();
    // if input has "/", we process it as a MultiAddr
    if input.contains('/') {
        let maddr = MultiAddr::from_str(input)?;
        if let Some(p) = maddr.iter().next() {
            match p.code() {
                Node::CODE => {
                    addr = p
                        .cast::<Node>()
                        .ok_or_else(|| ApiError::message("Failed to parse `node` protocol"))?
                        .to_string();
                }
                Service::CODE => {
                    addr = p
                        .cast::<Service>()
                        .ok_or_else(|| ApiError::message("Failed to parse `service` protocol"))?
                        .to_string();
                }
                Project::CODE => {
                    addr = p
                        .cast::<Project>()
                        .ok_or_else(|| ApiError::message("Failed to parse `project` protocol"))?
                        .to_string();
                }
                code => return Err(ApiError::message(format!("Protocol {code} not supported"))),
            }
        } else {
            return Err(ApiError::message("invalid address protocol"));
        }
    }
    if addr.is_empty() {
        return Err(ApiError::message(format!(
            "Empty address in input: {input}"
        )));
    }
    Ok(addr)
}

/// Replace the node's name with its address or leave it if it's another type of address.
///
/// Example:
///     if n1 has address of 127.0.0.1:1234
///     `/node/n1` -> `/ip4/127.0.0.1/tcp/1234`
pub async fn process_nodes_multiaddr(
    addr: &MultiAddr,
    cli_state: &CliState,
) -> miette::Result<MultiAddr> {
    let mut processed_addr = MultiAddr::default();
    for proto in addr.iter() {
        match proto.code() {
            Node::CODE => {
                let alias = proto
                    .cast::<Node>()
                    .ok_or_else(|| miette!("Invalid node address protocol"))?;
                let node_info = cli_state.get_node(&alias).await?;
                let addr = node_info.tcp_listener_multi_address()?;
                processed_addr.try_extend(&addr)?
            }
            _ => processed_addr.push_back_value(&proto)?,
        }
    }
    Ok(processed_addr)
}

pub fn get_free_address() -> Result<SocketAddr, ApiError> {
    get_free_address_for("127.0.0.1")
}

pub fn get_free_address_for(ip: &str) -> Result<SocketAddr, ApiError> {
    Ok(TcpListener::bind(format!("{ip}:0"))?.local_addr()?)
}
