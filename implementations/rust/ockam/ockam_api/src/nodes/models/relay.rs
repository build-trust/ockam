use colorful::Colorful;
use minicbor::{CborLen, Decode, Encode};

use ockam::identity::Identifier;
use ockam::remote::RemoteRelayInfo;
use ockam::route;
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::colors::OckamColor;
use crate::error::ApiError;
use crate::output::{colorize_connection_status, Output};
use crate::{route_to_multiaddr, ConnectionStatus};

/// Request body when instructing a node to create a relay
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateRelay {
    /// Address to create relay at.
    #[n(1)] pub(crate) address: MultiAddr,
    /// Relay alias.
    #[n(2)] pub(crate) alias: String,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(3)] pub(crate) authorized: Option<Identifier>,
    /// Relay address.
    #[n(4)] pub(crate) relay_address: Option<String>,
}

impl CreateRelay {
    pub fn new(
        address: MultiAddr,
        alias: String,
        auth: Option<Identifier>,
        relay_address: Option<String>,
    ) -> Self {
        Self {
            address,
            alias,
            authorized: auth,
            relay_address,
        }
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn authorized(&self) -> Option<Identifier> {
        self.authorized.clone()
    }

    pub fn relay_address(&self) -> Option<&str> {
        self.relay_address.as_deref()
    }
}

/// Response body when creating a relay
#[derive(Debug, Clone, Encode, Decode, CborLen, serde::Serialize, serde::Deserialize)]
#[rustfmt::skip]
#[cbor(map)]
pub struct RelayInfo {
    #[n(1)] forwarding_route: Option<String>,
    #[n(2)] remote_address: Option<String>,
    #[n(3)] worker_address: Option<String>,
    #[n(4)] flow_control_id: Option<FlowControlId>,
    #[n(5)] connection_status: ConnectionStatus,
    #[n(6)] destination_address: MultiAddr,
    #[n(7)] alias: String,
    #[n(8)] last_failure: Option<String>,
}

impl RelayInfo {
    pub fn new(
        destination_address: MultiAddr,
        alias: String,
        connection_status: ConnectionStatus,
    ) -> Self {
        Self {
            destination_address,
            alias,
            forwarding_route: None,
            remote_address: None,
            worker_address: None,
            flow_control_id: None,
            connection_status,
            last_failure: None,
        }
    }

    pub fn with(self, remote_relay_info: RemoteRelayInfo) -> Self {
        Self {
            forwarding_route: Some(remote_relay_info.forwarding_route().to_string()),
            remote_address: Some(remote_relay_info.remote_address().into()),
            worker_address: Some(remote_relay_info.worker_address().to_string()),
            flow_control_id: remote_relay_info.flow_control_id().clone(),
            connection_status: self.connection_status,
            destination_address: self.destination_address,
            alias: self.alias,
            last_failure: self.last_failure,
        }
    }

    pub fn with_last_failure(self, last_failure: String) -> Self {
        Self {
            forwarding_route: self.forwarding_route,
            remote_address: self.remote_address,
            worker_address: self.worker_address,
            flow_control_id: self.flow_control_id,
            connection_status: self.connection_status,
            destination_address: self.destination_address,
            alias: self.alias,
            last_failure: Some(last_failure),
        }
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        self.connection_status
    }

    pub fn destination_address(&self) -> &MultiAddr {
        &self.destination_address
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn forwarding_route(&self) -> &Option<String> {
        &self.forwarding_route
    }

    pub fn remote_address(&self) -> &Option<String> {
        &self.remote_address
    }

    pub fn flow_control_id(&self) -> &Option<FlowControlId> {
        &self.flow_control_id
    }

    pub fn remote_address_ma(&self) -> Result<Option<MultiAddr>, ockam_core::Error> {
        if let Some(addr) = &self.remote_address {
            route_to_multiaddr(&route![addr.to_string()])
                .ok_or_else(|| ApiError::core("Invalid Remote Address"))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn worker_address_ma(&self) -> Result<Option<MultiAddr>, ockam_core::Error> {
        if let Some(addr) = &self.worker_address {
            route_to_multiaddr(&route![addr.to_string()])
                .ok_or_else(|| ApiError::core("Invalid Worker Address"))
                .map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Output for RelayInfo {
    fn item(&self) -> crate::Result<String> {
        Ok(r#"
Relay:
    "#
        .to_owned()
            + self.as_list_item()?.as_str())
    }

    fn as_list_item(&self) -> crate::Result<String> {
        let output = format!(
            r#"Alias: {alias}
Status: {connection_status}
Remote Address: {remote_address}"#,
            alias = self.alias().color(OckamColor::PrimaryResource.color()),
            connection_status = colorize_connection_status(self.connection_status()),
            remote_address = self
                .remote_address_ma()?
                .map(|x| x.to_string())
                .unwrap_or("N/A".into())
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}
