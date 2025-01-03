use ockam::errcode::{Kind, Origin};
use ockam::transport::HostnamePort;
use ockam::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct InletConfig {
    pub inlet_change_history: String,
    pub inlet_identity_key: String,
    pub inlet_address: HostnamePort,
    pub relay_address: HostnamePort,
    pub outlet_identifier: String,
    pub outlet_relay_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct OutletConfig {
    pub outlet_change_history: String,
    pub outlet_identity_key: String,
    pub outlet_relay_name: String,
    pub outlet_peer_address: HostnamePort,
    pub tls: Option<bool>,
    pub relay_identifier: String,
    pub relay_address: HostnamePort,
    pub inlet_identifiers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RelayConfig {
    pub relay_change_history: String,
    pub relay_identity_key: String,
    pub outlet_identifier: String,
    pub relay_listener_address: HostnamePort,
}

pub fn parse<'a, T: Deserialize<'a>>(config: &'a str) -> Result<T> {
    serde_json::from_str(config).map_err(|err| {
        Error::new(
            Origin::Application,
            Kind::Misuse,
            format!("Invalid config json: {}", err),
        )
    })
}
