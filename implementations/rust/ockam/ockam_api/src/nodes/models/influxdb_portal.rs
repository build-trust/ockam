use std::time::Duration;

use minicbor::{CborLen, Decode, Encode};
use ockam_abac::PolicyExpression;
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
    
    #[n(2)] pub(crate) influxdb_org_id: String,
    #[n(3)] pub(crate) influxdb_token: String,
    #[n(4)] pub(crate)lease_permissions: String,
    #[n(5)] pub(crate)lease_usage: String,
    #[n(6)] pub(crate)expires_in: Duration,
}

impl CreateInfluxDBOutlet {
    pub fn new(
        tcp_outlet: CreateOutlet,
        influxdb_org_id: String,
        influxdb_token: String,
        lease_permissions: String,
        lease_usage: String,
        expires_in: Duration,
    ) -> Self {
        Self {
            tcp_outlet,
            influxdb_org_id,
            influxdb_token,
            lease_permissions,
            lease_usage,
            expires_in,
        }
    }
}
