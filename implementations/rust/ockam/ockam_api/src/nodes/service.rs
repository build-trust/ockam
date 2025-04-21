//! Node Manager (Node Man, the superhero that we deserve)

use ockam::Result;
use ockam_core::api::{RequestHeader, Response};

pub(crate) mod background_node_client;
pub mod default_address;
mod flow_controls;
pub(crate) mod in_memory_node;
pub mod kafka_services;
pub mod messages;
mod node_services;
pub(crate) mod policy;
mod projects;
pub mod relay;
mod secure_channel;
pub mod tcp_inlets;
pub mod tcp_outlets;
mod transport;
pub mod workers;

mod certificate_provider;
mod http;
mod interceptors;
mod manager;
mod trust;
mod worker;

pub use manager::*;
use ockam_core::Message;
pub use secure_channel::SecureChannelType;
pub use trust::*;
pub use worker::*;

const TARGET: &str = "ockam_api::nodemanager::service";

/// Append the request header to the response and serialize the response body
pub(crate) fn encode_response<T: Message>(
    req: &RequestHeader,
    res: std::result::Result<Response<T>, Response<ockam_core::api::Error>>,
) -> Result<Response<Vec<u8>>> {
    let v = match res {
        Ok(r) => r.with_headers(req).encode_body()?,
        Err(e) => e.with_headers(req).encode_body()?,
    };
    Ok(v)
}
