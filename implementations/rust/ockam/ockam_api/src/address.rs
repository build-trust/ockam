use crate::error::{ApiError, ParseError};
use ockam_core::env::{get_env_with_default, FromString};
use ockam_core::Result;
use ockam_multiaddr::proto::{Node, Project, Service};
use ockam_multiaddr::{MultiAddr, Protocol};
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;

pub const OCKAM_CONTROLLER_ADDR: &str = "OCKAM_CONTROLLER_ADDR";
pub const DEFAULT_CONTROLLER_ADDRESS: &str = "/dnsaddr/orchestrator.ockam.io/tcp/6252/service/api";

pub fn controller_route() -> MultiAddr {
    let default_addr = MultiAddr::from_string(DEFAULT_CONTROLLER_ADDRESS)
        .unwrap_or_else(|_| panic!("invalid Controller route: {DEFAULT_CONTROLLER_ADDRESS}"));

    let route = get_env_with_default::<MultiAddr>(OCKAM_CONTROLLER_ADDR, default_addr).unwrap();
    trace!(%route, "Controller route");

    route
}

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
                        .ok_or(ApiError::message("Failed to parse `node` protocol"))?
                        .to_string();
                }
                Service::CODE => {
                    addr = p
                        .cast::<Service>()
                        .ok_or(ApiError::message("Failed to parse `service` protocol"))?
                        .to_string();
                }
                Project::CODE => {
                    addr = p
                        .cast::<Project>()
                        .ok_or(ApiError::message("Failed to parse `project` protocol"))?
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

pub fn get_free_address() -> Result<SocketAddr, ApiError> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let res = format!("127.0.0.1:{port}")
        .parse()
        .map_err(ParseError::from)?;
    Ok(res)
}
