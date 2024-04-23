use std::net::SocketAddr;

use serde::Serialize;

use crate::Error;
use ockam_api::{
    nodes::models::portal::{InletStatus, OutletStatus},
    route_to_multiaddr, try_address_to_multiaddr, ConnectionStatus,
};
use ockam_core::Route;
use ockam_multiaddr::MultiAddr;

/// Information to display of the inlets in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowInletStatus {
    pub status: ConnectionStatus,
    pub listen_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_to_outlet: Option<MultiAddr>,
}

impl From<InletStatus> for ShowInletStatus {
    fn from(inlet: InletStatus) -> Self {
        Self {
            status: inlet.status,
            listen_address: inlet.bind_addr,
            route_to_outlet: inlet
                .outlet_route
                .and_then(Route::parse)
                .and_then(|r| route_to_multiaddr(&r)),
        }
    }
}

/// Information to display of the inlets in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowOutletStatus {
    pub forward_address: SocketAddr,
    pub address: MultiAddr,
}

impl TryFrom<OutletStatus> for ShowOutletStatus {
    type Error = Error;

    fn try_from(value: OutletStatus) -> Result<Self, Self::Error> {
        Ok(Self {
            forward_address: value.socket_addr,
            address: try_address_to_multiaddr(&value.worker_addr)?,
        })
    }
}
