use std::{net::SocketAddr, str::FromStr};

use ockam_api::{port_range::PortRange, DefaultAddress};
use ockam_multiaddr::MultiAddr;

pub(crate) mod client;
pub(crate) mod outlet;
pub(crate) mod util;

const KAFKA_DEFAULT_BOOTSTRAP_ADDRESS: &str = "localhost:9092";
const KAFKA_DEFAULT_PROJECT_ROUTE: &str = "/project/default";
const KAFKA_DEFAULT_CLIENT_SERVER: &str = "127.0.0.1:4000";
const KAFKA_DEFAULT_CLIENT_PORT_RANGE: &str = "4001-4100";

fn kafka_default_outlet_addr() -> String {
    DefaultAddress::KAFKA_OUTLET.to_string()
}

fn kafka_consumer_default_addr() -> String {
    DefaultAddress::KAFKA_CLIENT.to_string()
}

fn kafka_default_project_route() -> MultiAddr {
    MultiAddr::from_str(KAFKA_DEFAULT_PROJECT_ROUTE).expect("Failed to parse default project route")
}

fn kafka_default_outlet_server() -> String {
    KAFKA_DEFAULT_BOOTSTRAP_ADDRESS.to_string()
}

fn kafka_default_consumer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_CLIENT_SERVER)
        .expect("Failed to parse default consumer server")
}

fn kafka_default_consumer_port_range() -> PortRange {
    PortRange::from_str(KAFKA_DEFAULT_CLIENT_PORT_RANGE)
        .expect("Failed to parse default consumer port range")
}
