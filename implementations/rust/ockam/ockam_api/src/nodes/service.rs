//! Node Manager (Node Man, the superhero that we deserve)

use minicbor::Encode;

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
pub mod portals;
mod projects;
pub mod relay;
mod secure_channel;
mod transport;
mod udp_puncture;
pub mod workers;

mod http;
mod manager;
mod trust;
mod worker;

pub use manager::*;
pub use secure_channel::SecureChannelType;
pub use trust::*;
pub use worker::*;

const TARGET: &str = "ockam_api::nodemanager::service";

/// Append the request header to the Response and encode in vector format
pub(crate) fn encode_response<T: Encode<()>>(
    req: &RequestHeader,
    res: std::result::Result<Response<T>, Response<ockam_core::api::Error>>,
) -> Result<Vec<u8>> {
    let v = match res {
        Ok(r) => r.with_headers(req).to_vec()?,
        Err(e) => e.with_headers(req).to_vec()?,
    };

    Ok(v)
}
