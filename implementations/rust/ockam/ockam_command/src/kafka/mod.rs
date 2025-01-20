use ockam::transport::{HostnamePort, SchemeHostnamePort, StaticHostnamePort};
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::port_range::PortRange;
use ockam_multiaddr::MultiAddr;
use std::cmp::min;
use std::str::FromStr;

pub(crate) mod consumer;
pub(crate) mod inlet;
pub(crate) mod outlet;
pub(crate) mod producer;
pub(crate) mod util;

const KAFKA_DEFAULT_CONSUMER_SERVER: StaticHostnamePort =
    StaticHostnamePort::new("127.0.0.1", 4000);
const KAFKA_DEFAULT_PRODUCER_SERVER: StaticHostnamePort =
    StaticHostnamePort::new("127.0.0.1", 5000);

fn kafka_default_consumer_server() -> SchemeHostnamePort {
    KAFKA_DEFAULT_CONSUMER_SERVER.try_into().unwrap()
}
fn kafka_default_producer_server() -> SchemeHostnamePort {
    KAFKA_DEFAULT_PRODUCER_SERVER.try_into().unwrap()
}

const KAFKA_DEFAULT_BOOTSTRAP_ADDRESS: StaticHostnamePort =
    StaticHostnamePort::new("127.0.0.1", 9092);
const KAFKA_DEFAULT_PROJECT_ROUTE: &str = "/project/default";
const KAFKA_DEFAULT_INLET_BIND_ADDRESS: StaticHostnamePort =
    StaticHostnamePort::new("127.0.0.1", 4000);

fn kafka_default_outlet_addr() -> String {
    DefaultAddress::KAFKA_OUTLET.to_string()
}

fn kafka_inlet_default_addr() -> String {
    DefaultAddress::KAFKA_INLET.to_string()
}

fn kafka_default_project_route() -> MultiAddr {
    MultiAddr::from_str(KAFKA_DEFAULT_PROJECT_ROUTE).expect("Failed to parse default project route")
}

fn kafka_default_outlet_server() -> SchemeHostnamePort {
    KAFKA_DEFAULT_BOOTSTRAP_ADDRESS.try_into().unwrap()
}

fn kafka_default_inlet_bind_address() -> SchemeHostnamePort {
    KAFKA_DEFAULT_INLET_BIND_ADDRESS.try_into().unwrap()
}

pub(crate) fn make_brokers_port_range<T: Into<HostnamePort>>(bootstrap_server: T) -> PortRange {
    let boostrap_server_port = bootstrap_server.into().port() as u32;
    let start = min(boostrap_server_port + 1, u16::MAX as u32) as u16;
    let end = min(boostrap_server_port + 100, u16::MAX as u32) as u16;
    // we can unwrap here because we know that range start <= range end
    PortRange::new(start, end).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brokers_port_range() {
        let address = SchemeHostnamePort::from_str("tls://127.0.0.1:8080").unwrap();
        let port_range = make_brokers_port_range(address);
        assert_eq!(port_range.start(), 8081);
        assert_eq!(port_range.end(), 8180);

        let address = HostnamePort::from_str("127.0.0.1:8080").unwrap();
        let port_range = make_brokers_port_range(address);
        assert_eq!(port_range.start(), 8081);
        assert_eq!(port_range.end(), 8180);

        let address = HostnamePort::from_str("127.0.0.1:65442").unwrap();
        let port_range = make_brokers_port_range(address);
        assert_eq!(port_range.start(), 65443);
        assert_eq!(port_range.end(), u16::MAX);
    }
}
