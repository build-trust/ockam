use std::{net::SocketAddr, str::FromStr};

use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::port_range::PortRange;
use ockam_multiaddr::MultiAddr;

pub(crate) mod consumer;
pub(crate) mod inlet;
pub(crate) mod outlet;
pub(crate) mod producer;
pub(crate) mod util;

const KAFKA_DEFAULT_BOOTSTRAP_ADDRESS: &str = "127.0.0.1:9092";
const KAFKA_DEFAULT_PROJECT_ROUTE: &str = "/project/default";
const KAFKA_DEFAULT_CONSUMER_SERVER: &str = "127.0.0.1:4000";
const KAFKA_DEFAULT_INLET_BIND_ADDRESS: &str = "127.0.0.1:4000";
const KAFKA_DEFAULT_PRODUCER_SERVER: &str = "127.0.0.1:5000";

fn kafka_default_outlet_addr() -> String {
    DefaultAddress::KAFKA_OUTLET.to_string()
}

fn kafka_inlet_default_addr() -> String {
    DefaultAddress::KAFKA_INLET.to_string()
}

fn kafka_default_project_route() -> MultiAddr {
    MultiAddr::from_str(KAFKA_DEFAULT_PROJECT_ROUTE).expect("Failed to parse default project route")
}

fn kafka_default_outlet_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_BOOTSTRAP_ADDRESS)
        .expect("Failed to parse default bootstrap address")
}

fn kafka_default_consumer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_CONSUMER_SERVER)
        .expect("Failed to parse default consumer server")
}

fn kafka_default_inlet_bind_address() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_INLET_BIND_ADDRESS)
        .expect("Failed to parse default consumer server")
}

fn kafka_default_producer_server() -> SocketAddr {
    SocketAddr::from_str(KAFKA_DEFAULT_PRODUCER_SERVER)
        .expect("Failed to parse default producer server")
}

pub(crate) fn make_brokers_port_range(bootstrap_server: &SocketAddr) -> PortRange {
    let boostrap_server_port = bootstrap_server.port();
    // we can unwrap here because we know that range start <= range end
    PortRange::new(boostrap_server_port + 1, boostrap_server_port + 100).unwrap()
}
