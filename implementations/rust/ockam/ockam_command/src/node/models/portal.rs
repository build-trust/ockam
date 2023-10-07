use std::net::SocketAddr;

use ockam_api::{
    addr_to_multiaddr,
    nodes::models::portal::{InletStatus, OutletStatus},
    route_to_multiaddr,
};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

/// Information to display of the inlets in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowInletStatus {
    pub listen_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_to_outlet: Option<MultiAddr>,
}

impl From<InletStatus> for ShowInletStatus {
    fn from(value: InletStatus) -> Self {
        Self {
            listen_address: value.bind_addr,
            route_to_outlet: Route::parse(value.outlet_route).and_then(|r| route_to_multiaddr(&r)),
        }
    }
}

/// Information to display of the inlets in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowOutletStatus {
    pub forward_address: SocketAddr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<MultiAddr>,
}

impl From<OutletStatus> for ShowOutletStatus {
    fn from(value: OutletStatus) -> Self {
        Self {
            forward_address: value.socket_addr,
            address: addr_to_multiaddr(value.worker_addr),
        }
    }
}
