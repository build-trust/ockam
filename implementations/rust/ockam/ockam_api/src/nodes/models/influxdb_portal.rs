use minicbor::{CborLen, Decode, Encode};
use ockam_multiaddr::MultiAddr;

use super::portal::{CreateInlet, CreateOutlet};

/// Request body to create an influxdb inlet
#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInfluxDBInlet {
    /// The address the portal should listen at.
    #[n(1)] pub(crate) tcp_inlet: CreateInlet,
    /// The token leaser service address.
    #[n(2)] pub(crate) service_address: MultiAddr,
}

impl CreateInfluxDBInlet {
    pub fn new(tcp_inlet: CreateInlet, service_address: MultiAddr) -> Self {
        Self {
            tcp_inlet,
            service_address,
        }
    }
}

/// Request body to create an influxdb outlet
#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateInfluxDBOutlet {
    /// The address the portal should listen at.
    #[n(1)] pub(crate) tcp_outlet: CreateOutlet,
    /// The token leaser service address.
    #[n(2)] pub(crate) service_address: MultiAddr,
}

impl CreateInfluxDBOutlet {
    pub fn new(tcp_outlet: CreateOutlet, service_address: MultiAddr) -> Self {
        Self {
            tcp_outlet,
            service_address,
        }
    }
}
