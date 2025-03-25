use crate::control_api::protocol::common::ConnectionStatus;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateRelayRequest {
    /// Name of Relay;
    /// Whe omitted, a random name will be generated
    pub name: Option<String>,
    /// Route to the node that will be used as a relay;
    #[schema(example = "/project/default")]
    pub to: String,
    /// The address of the relay;
    /// The resulting address in the relay will be `forward_to_{address}`;
    /// When omitted, the name will be used;
    pub address: Option<String>,
    /// Only allows relay node with the provided identity;
    /// When omitted, all identities are allowed;
    #[schema(example = "Id3b788c6a89de8b1f2fd13743eb3123178cf6ec7c9253be8ddcf7e154abe016a")]
    pub authorized: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RelayStatus {
    /// Name of the relay
    pub name: String,
    /// Multiaddress to the node that is used as a relay
    pub to: String,
    /// The address of the relay within the relay;
    /// Usually it follows the form `forward_to_{address}`
    pub remote_address: Option<String>,
    /// The status of the relay
    pub status: ConnectionStatus,
}

impl From<crate::nodes::models::relay::RelayInfo> for RelayStatus {
    fn from(info: crate::nodes::models::relay::RelayInfo) -> Self {
        Self {
            name: info.name,
            to: info.destination_address.to_string(),
            remote_address: info.remote_address.map(|addr| addr.to_string()),
            status: info.connection_status.into(),
        }
    }
}
