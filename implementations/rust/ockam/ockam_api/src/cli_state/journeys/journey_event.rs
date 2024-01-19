use opentelemetry::Key;
use std::fmt::{Display, Formatter};

/// List of attribute keys for journey event creation

pub const TCP_OUTLET_AT: &Key = &Key::from_static_str("app.tcp_outlet.at");
pub const TCP_OUTLET_FROM: &Key = &Key::from_static_str("app.tcp_outlet.from");
pub const TCP_OUTLET_TO: &Key = &Key::from_static_str("app.tcp_outlet.to");
pub const TCP_OUTLET_ALIAS: &Key = &Key::from_static_str("app.tcp_outlet.alias");

/// List of all the journey events that we want to track
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JourneyEvent {
    Enrolled,
    NodeCreated,
    TcpInletCreated,
    TcpOutletCreated,
    RelayCreated,
    PortalCreated,
    Error {
        command_name: String,
        message: String,
    },
}

impl Display for JourneyEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            JourneyEvent::Enrolled => f.write_str("enrolled"),
            JourneyEvent::NodeCreated => f.write_str("node created"),
            JourneyEvent::TcpInletCreated => f.write_str("tcp inlet created"),
            JourneyEvent::TcpOutletCreated => f.write_str("tcp outlet created"),
            JourneyEvent::RelayCreated => f.write_str("relay created"),
            JourneyEvent::PortalCreated => f.write_str("portal created"),
            JourneyEvent::Error { command_name, .. } => {
                f.write_fmt(format_args!("{} error", command_name))
            }
        }
    }
}
