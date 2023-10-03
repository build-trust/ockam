use std::fmt::Display;

use colorful::Colorful;

use ockam_multiaddr::{
    proto::{DnsAddr, Node, Tcp},
    MultiAddr,
};
use serde::Serialize;

use crate::output::Output;

use super::{
    portal::{ShowInletStatus, ShowOutletStatus},
    secure_channel::ShowSecureChannelListener,
    services::ShowServiceStatus,
    transport::ShowTransportStatus,
};

/// Information to display in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowNodeResponse {
    pub is_default: bool,
    pub name: String,
    pub is_up: bool,
    pub route: RouteToNode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    pub transports: Vec<ShowTransportStatus>,
    pub secure_channel_listeners: Vec<ShowSecureChannelListener>,
    pub inlets: Vec<ShowInletStatus>,
    pub outlets: Vec<ShowOutletStatus>,
    pub services: Vec<ShowServiceStatus>,
}
#[derive(Debug, Serialize)]
pub struct RouteToNode {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<MultiAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<MultiAddr>,
}

impl ShowNodeResponse {
    pub fn new(
        is_default: bool,
        name: &str,
        is_up: bool,
        node_port: Option<u16>,
    ) -> ShowNodeResponse {
        let mut m = MultiAddr::default();
        let short = m.push_back(Node::new(name)).ok().map(|_| m);

        let verbose = node_port.and_then(|port| {
            let mut m = MultiAddr::default();
            if m.push_back(DnsAddr::new("localhost")).is_ok() && m.push_back(Tcp::new(port)).is_ok()
            {
                Some(m)
            } else {
                None
            }
        });

        ShowNodeResponse {
            is_default,
            name: name.to_owned(),
            is_up,
            route: RouteToNode { short, verbose },
            identity: None,
            transports: Default::default(),
            secure_channel_listeners: Default::default(),
            inlets: Default::default(),
            outlets: Default::default(),
            services: Default::default(),
        }
    }
}

impl Display for ShowNodeResponse {
    fn fmt(&self, buffer: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(buffer, "Node:")?;

        if self.is_default {
            writeln!(buffer, "  Name: {} (default)", self.name)?;
        } else {
            writeln!(buffer, "  Name: {}", self.name)?;
        }

        writeln!(
            buffer,
            "  Status: {}",
            match self.is_up {
                true => "UP".light_green(),
                false => "DOWN".light_red(),
            }
        )?;

        writeln!(buffer, "  Route To Node:")?;
        if let Some(short) = &self.route.short {
            writeln!(buffer, "    Short: {short}")?;
        }
        if let Some(verbose) = &self.route.verbose {
            writeln!(buffer, "    Verbose: {verbose}")?;
        }

        if let Some(identity) = &self.identity {
            writeln!(buffer, "  Identity: {}", identity)?;
        }

        writeln!(buffer, "  Transports:")?;
        for e in &self.transports {
            writeln!(buffer, "    Transport:")?;
            writeln!(buffer, "      Type: {}", &e.tt)?;
            writeln!(buffer, "      Mode: {}", &e.mode)?;
            writeln!(buffer, "      Socket: {}", &e.socket)?;
            writeln!(buffer, "      Worker: {}", &e.worker)?;
            writeln!(buffer, "      FlowControlId: {}", &e.flow_control)?;
        }

        writeln!(buffer, "  Secure Channel Listeners:")?;
        for e in &self.secure_channel_listeners {
            writeln!(buffer, "    Listener:")?;
            if let Some(ma) = &e.address {
                writeln!(buffer, "      Address: {ma}")?;
            }
            writeln!(buffer, "      FlowControlId: {}", &e.flow_control)?;
        }

        writeln!(buffer, "  Inlets:")?;
        for e in &self.inlets {
            writeln!(buffer, "    Inlet:")?;
            writeln!(buffer, "      Listen Address: {}", e.listen_address)?;
            if let Some(r) = &e.route_to_outlet {
                writeln!(buffer, "      Route To Outlet: {r}")?;
            }
        }

        writeln!(buffer, "  Outlets:")?;
        for e in &self.outlets {
            writeln!(buffer, "    Outlet:")?;
            writeln!(buffer, "      Forward Address: {}", e.forward_address)?;
            if let Some(ma) = &e.address {
                writeln!(buffer, "      Address: {ma}")?;
            }
        }

        writeln!(buffer, "  Services:")?;
        for e in &self.services {
            writeln!(buffer, "    Service:")?;
            writeln!(buffer, "      Type: {}", e.service_type)?;
            if let Some(ma) = &e.address {
                writeln!(buffer, "      Address: {ma}")?;
            }
        }

        Ok(())
    }
}

impl Output for ShowNodeResponse {
    fn output(&self) -> crate::error::Result<String> {
        Ok(self.to_string())
    }
}
