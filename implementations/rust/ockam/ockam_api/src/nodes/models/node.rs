//! Nodemanager API types

use crate::cli_state::{NodeInfo, NodeProcessStatus};
use crate::colors::color_primary;
use crate::nodes::models::portal::{InletStatus, OutletStatus};
use crate::nodes::models::services::ServiceStatus;
use crate::nodes::models::transport::TransportStatus;
use crate::output::Output;
use crate::terminal::fmt;
use minicbor::{CborLen, Decode, Encode};
use ockam::identity::{Identifier, SecureChannelListener};
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;

use crate::config::lookup::InternetAddress;
use std::fmt::{Display, Formatter};

/// Response body for a node status request
#[derive(Debug, Clone, Serialize, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeStatus {
    #[n(1)] pub name: String,
    #[n(2)] pub identifier: Identifier,
    #[n(3)] pub status: NodeProcessStatus,
}

impl NodeStatus {
    pub fn new(name: impl Into<String>, identifier: Identifier, status: NodeProcessStatus) -> Self {
        Self {
            name: name.into(),
            identifier,
            status,
        }
    }
}

impl From<&NodeInfo> for NodeStatus {
    fn from(node: &NodeInfo) -> Self {
        Self {
            name: node.name(),
            identifier: node.identifier(),
            status: node.status(),
        }
    }
}

#[derive(Debug, Serialize, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct NodeResources {
    #[n(1)] pub name: String,
    #[n(2)] pub identity_name: String,
    #[n(3)] pub is_default: bool,
    #[serde(flatten)]
    #[n(4)] pub status: NodeProcessStatus,
    #[n(5)] pub route: RouteToNode,
    #[n(6)] pub http_server_address: Option<InternetAddress>,
    #[n(7)] pub transports: Vec<TransportStatus>,
    #[n(8)] pub secure_channel_listeners: Vec<SecureChannelListener>,
    #[n(9)] pub inlets: Vec<InletStatus>,
    #[n(10)] pub outlets: Vec<OutletStatus>,
    #[n(11)] pub services: Vec<ServiceStatus>,
}

#[allow(clippy::too_many_arguments)]
impl NodeResources {
    pub fn from_parts(
        node: NodeInfo,
        identity_name: String,
        transports: Vec<TransportStatus>,
        listeners: Vec<SecureChannelListener>,
        inlets: Vec<InletStatus>,
        outlets: Vec<OutletStatus>,
        services: Vec<ServiceStatus>,
    ) -> Result<Self> {
        Ok(Self {
            name: node.name(),
            identity_name,
            is_default: node.is_default(),
            status: node.status(),
            route: RouteToNode {
                short: node.route()?,
                verbose: node.verbose_route()?,
            },
            http_server_address: node.http_server_address(),
            transports,
            secure_channel_listeners: listeners,
            inlets,
            outlets,
            services,
        })
    }

    pub fn empty(node: NodeInfo, identity_name: String) -> Result<Self> {
        Ok(Self {
            name: node.name(),
            identity_name,
            is_default: node.is_default(),
            status: node.status(),
            route: RouteToNode {
                short: node.route()?,
                verbose: node.verbose_route()?,
            },
            http_server_address: None,
            transports: vec![],
            secure_channel_listeners: vec![],
            inlets: vec![],
            outlets: vec![],
            services: vec![],
        })
    }
}

impl Display for NodeResources {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", fmt::PADDING, color_primary(&self.name))?;
        if self.is_default {
            write!(f, " (default)")?;
        }
        writeln!(f, ":")?;

        writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION, self.status)?;
        writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION, self.route)?;
        if let Some(http_server) = self.http_server_address.as_ref() {
            writeln!(
                f,
                "{}{}HTTP server listening at {}",
                fmt::PADDING,
                fmt::INDENTATION,
                color_primary(http_server.to_string())
            )?;
        }

        writeln!(
            f,
            "{}{}Identity: {}",
            fmt::PADDING,
            fmt::INDENTATION,
            color_primary(&self.identity_name)
        )?;

        if self.transports.is_empty() {
            writeln!(f, "{}{}No Transports", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}{}Transports:", fmt::PADDING, fmt::INDENTATION)?;
            for t in &self.transports {
                writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION.repeat(2), t)?;
            }
        }

        if self.secure_channel_listeners.is_empty() {
            writeln!(f, "{}{}No Secure Channels", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}{}Secure Channels:", fmt::PADDING, fmt::INDENTATION)?;
            for s in &self.secure_channel_listeners {
                writeln!(
                    f,
                    "{}{}{}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    s.item().map_err(|_| std::fmt::Error)?
                )?;
            }
        }

        if self.inlets.is_empty() && self.outlets.is_empty() {
            writeln!(f, "{}{}No Portals", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}{}Portals:", fmt::PADDING, fmt::INDENTATION)?;
            for i in &self.inlets {
                writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION.repeat(2), i)?;
            }

            for o in &self.outlets {
                writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION.repeat(2), o)?;
            }
        }

        if self.services.is_empty() {
            writeln!(f, "{}{}No Services", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(f, "{}{}Services:", fmt::PADDING, fmt::INDENTATION)?;
            for s in &self.services {
                writeln!(f, "{}{}{}", fmt::PADDING, fmt::INDENTATION.repeat(2), s)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct RouteToNode {
    #[n(1)] pub short: MultiAddr,
    #[n(2)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<MultiAddr>,
}

impl Display for RouteToNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "With route {}", color_primary(self.short.to_string()))?;
        if let Some(verbose) = &self.verbose {
            write!(f, " or {}", color_primary(verbose.to_string()))?;
        }
        Ok(())
    }
}
