use std::fmt::Display;

use ockam_api::colors::{color_primary, color_warn};
use ockam_api::ConnectionStatus;
use serde::Serialize;

use ockam_multiaddr::{
    proto::{DnsAddr, Tcp},
    MultiAddr,
};

use ockam_api::output::Output;
use ockam_api::terminal::fmt;
use ockam_multiaddr::proto::Node;

use super::{
    portal::{ShowInletStatus, ShowOutletStatus},
    secure_channel::ShowSecureChannelListener,
    services::ShowServiceStatus,
    transport::ShowTransportStatus,
};
use crate::Result;

/// Information to display in the `ockam node show` command
#[derive(Debug, Serialize)]
pub struct ShowNodeResponse {
    pub is_default: bool,
    pub name: String,
    pub status: ConnectionStatus,
    pub node_pid: Option<u32>,
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
    pub short: MultiAddr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<MultiAddr>,
}

impl ShowNodeResponse {
    pub fn new(
        is_default: bool,
        name: &str,
        is_up: bool,
        node_port: Option<u16>,
        node_pid: Option<u32>,
    ) -> Result<ShowNodeResponse> {
        let short = {
            let mut m = MultiAddr::default();
            m.push_back(Node::new(name))?;
            m
        };
        let verbose = if let Some(port) = node_port {
            let mut m = MultiAddr::default();
            m.push_back(DnsAddr::new("localhost"))?;
            m.push_back(Tcp::new(port))?;
            Some(m)
        } else {
            None
        };

        Ok(ShowNodeResponse {
            is_default,
            name: name.to_owned(),
            status: if is_up {
                ConnectionStatus::Up
            } else {
                ConnectionStatus::Down
            },
            node_pid,
            route: RouteToNode { short, verbose },
            identity: None,
            transports: Default::default(),
            secure_channel_listeners: Default::default(),
            inlets: Default::default(),
            outlets: Default::default(),
            services: Default::default(),
        })
    }

    pub fn is_up(&self) -> bool {
        self.status == ConnectionStatus::Up
    }
}

impl Display for ShowNodeResponse {
    fn fmt(&self, buffer: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(buffer, "{}{}", fmt::PADDING, color_primary(&self.name))?;
        if self.is_default {
            write!(buffer, " (default)")?;
        }
        writeln!(buffer, ":")?;

        write!(
            buffer,
            "{}{}The node is {}",
            fmt::PADDING,
            fmt::INDENTATION,
            self.status
        )?;
        if let Some(node_pid) = self.node_pid {
            write!(buffer, ", with PID {}", node_pid)?;
        }
        writeln!(buffer)?;

        write!(
            buffer,
            "{}{}With route {}",
            fmt::PADDING,
            fmt::INDENTATION,
            color_primary(self.route.short.to_string())
        )?;
        if let Some(verbose) = &self.route.verbose {
            write!(buffer, " or {}", color_primary(verbose.to_string()))?;
        }
        writeln!(buffer)?;

        if let Some(identity) = &self.identity {
            writeln!(
                buffer,
                "{}{}Identity: {}",
                fmt::PADDING,
                fmt::INDENTATION,
                identity
            )?;
        }

        if self.transports.is_empty() {
            writeln!(buffer, "{}{}No Transports", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(buffer, "{}{}Transports:", fmt::PADDING, fmt::INDENTATION)?;
            for t in &self.transports {
                writeln!(
                    buffer,
                    "{}{}{}, {} at {}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    t.tt,
                    t.mode,
                    color_primary(&t.socket)
                )?;
            }
        }

        if self.secure_channel_listeners.is_empty() {
            writeln!(
                buffer,
                "{}{}No Secure Channels",
                fmt::PADDING,
                fmt::INDENTATION
            )?;
        } else {
            writeln!(
                buffer,
                "{}{}Secure Channels:",
                fmt::PADDING,
                fmt::INDENTATION
            )?;
            for s in &self.secure_channel_listeners {
                writeln!(
                    buffer,
                    "{}{}Listener at {}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    color_primary(&s.address.to_string())
                )?;
            }
        }

        if self.inlets.is_empty() && self.outlets.is_empty() {
            writeln!(buffer, "{}{}No Portals", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(buffer, "{}{}Portals:", fmt::PADDING, fmt::INDENTATION)?;
            for i in &self.inlets {
                write!(
                    buffer,
                    "{}{}Inlet at {} is {}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    color_primary(&i.listen_address),
                    i.status,
                )?;
                if let Some(r) = &i.route_to_outlet {
                    writeln!(
                        buffer,
                        " with route to outlet {}",
                        color_primary(r.to_string())
                    )?;
                } else {
                    writeln!(buffer)?;
                }
            }

            for o in &self.outlets {
                writeln!(
                    buffer,
                    "{}{}Outlet {} at {}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    color_primary(o.address.to_string()),
                    color_primary(&o.forward_address.to_string()),
                )?;
            }
        }
        if self.services.is_empty() {
            writeln!(buffer, "{}{}No Services", fmt::PADDING, fmt::INDENTATION)?;
        } else {
            writeln!(buffer, "{}{}Services:", fmt::PADDING, fmt::INDENTATION)?;
            for s in &self.services {
                writeln!(
                    buffer,
                    "{}{}{} service at {}",
                    fmt::PADDING,
                    fmt::INDENTATION.repeat(2),
                    color_warn(&s.service_type),
                    color_primary(&s.address.to_string())
                )?;
            }
        }

        Ok(())
    }
}

impl Output for ShowNodeResponse {
    fn single(&self) -> ockam_api::Result<String> {
        Ok(self.to_string())
    }
}
