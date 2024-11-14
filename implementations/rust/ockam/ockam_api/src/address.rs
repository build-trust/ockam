use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;

use ockam_core::Result;
use ockam_multiaddr::proto::{Node, Project, Service};
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::error::ApiError;

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

pub fn get_free_address() -> Result<SocketAddr, ApiError> {
    get_free_address_for("127.0.0.1")
}

pub fn get_free_address_for(ip: &str) -> Result<SocketAddr, ApiError> {
    Ok(TcpListener::bind(format!("{ip}:0"))?.local_addr()?)
}
