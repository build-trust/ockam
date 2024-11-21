use minicbor::{CborLen, Decode, Encode};
use std::fmt::Display;

use ockam::identity::Identifier;
use ockam::remote::RemoteRelayInfo;
use ockam::route;
use ockam_core::flow_control::FlowControlId;
use ockam_multiaddr::MultiAddr;

use crate::colors::color_primary;
use crate::output::Output;
use crate::session::replacer::ReplacerOutputKind;
use crate::session::session::Session;
use crate::{ConnectionStatus, ReverseLocalConverter};

#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum ReturnTiming {
    #[n(1)] Immediately,
    #[n(2)] AfterConnection,
}

/// Request body when instructing a node to create a relay
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateRelay {
    /// Address to create relay at.
    #[n(1)] pub(crate) address: MultiAddr,
    /// Relay name.
    #[n(2)] pub(crate) name: String,
    /// An authorised identity for secure channels.
    /// Only set for non-project addresses as for projects the project's
    /// authorised identity will be used.
    #[n(3)] pub(crate) authorized: Option<Identifier>,
    /// Relay address.
    #[n(4)] pub(crate) relay_address: Option<String>,
    /// When to return.
    #[n(5)] pub(crate) return_timing: ReturnTiming,
}

impl CreateRelay {
    pub fn new(
        address: MultiAddr,
        name: String,
        auth: Option<Identifier>,
        relay_address: Option<String>,
        return_timing: ReturnTiming,
    ) -> Self {
        Self {
            address,
            name,
            authorized: auth,
            relay_address,
            return_timing,
        }
    }

    pub fn address(&self) -> &MultiAddr {
        &self.address
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn authorized(&self) -> Option<Identifier> {
        self.authorized.clone()
    }

    pub fn relay_address(&self) -> Option<&str> {
        self.relay_address.as_deref()
    }

    pub fn return_timing(&self) -> ReturnTiming {
        self.return_timing.clone()
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
    #[n(7)] name: String,
    #[n(8)] last_failure: Option<String>,
}

impl RelayInfo {
    pub fn new(
        destination_address: MultiAddr,
        name: String,
        connection_status: ConnectionStatus,
    ) -> Self {
        Self {
            destination_address,
            name,
            forwarding_route: None,
            remote_address: None,
            worker_address: None,
            flow_control_id: None,
            connection_status,
            last_failure: None,
        }
    }

    pub fn from_session(session: &Session, destination_address: MultiAddr, name: String) -> Self {
        let relay_info = Self::new(destination_address, name, session.connection_status());
        if let Some(outcome) = session.last_outcome() {
            match outcome {
                ReplacerOutputKind::Relay(info) => relay_info.with(info),
                ReplacerOutputKind::Inlet(_) => {
                    panic!("InletInfo should not be in the registry")
                }
            }
        } else {
            relay_info
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
            name: self.name,
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
            name: self.name,
            last_failure: Some(last_failure),
        }
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        self.connection_status
    }

    pub fn destination_address(&self) -> &MultiAddr {
        &self.destination_address
    }

    pub fn name(&self) -> &str {
        &self.name
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
            ReverseLocalConverter::convert_route(&route![addr.to_string()]).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn worker_address_ma(&self) -> Result<Option<MultiAddr>, ockam_core::Error> {
        if let Some(addr) = &self.worker_address {
            ReverseLocalConverter::convert_route(&route![addr.to_string()]).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Display for RelayInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Relay {} is {} at {}",
            color_primary(&self.name),
            self.connection_status(),
            color_primary(self.destination_address.to_string())
        )?;
        writeln!(
            f,
            "With address {}",
            color_primary(
                self.remote_address_ma()
                    .unwrap_or(None)
                    .map(|x| x.to_string())
                    .unwrap_or("N/A".into())
            ),
        )?;
        Ok(())
    }
}

impl Output for RelayInfo {
    fn item(&self) -> crate::Result<String> {
        Ok(self.padded_display())
    }
}
