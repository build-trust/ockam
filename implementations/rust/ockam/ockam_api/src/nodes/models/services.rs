use minicbor::{Decode, Encode};
use ockam_core::compat::net::SocketAddr;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;

use serde::Serialize;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartServiceRequest<T> {
    #[n(1)] addr: String,
    #[n(2)] req: T,
}

impl<T> StartServiceRequest<T> {
    pub fn new<S: Into<String>>(req: T, addr: S) -> Self {
        Self {
            addr: addr.into(),
            req,
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }

    pub fn request(&self) -> &T {
        &self.req
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DeleteServiceRequest {
    #[n(1)] addr: String,
}

impl DeleteServiceRequest {
    pub fn new<S: Into<String>>(addr: S) -> Self {
        Self { addr: addr.into() }
    }

    pub fn address(&self) -> Address {
        Address::from(self.addr.clone())
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaOutletRequest {
    #[n(1)] pub bootstrap_server_addr: SocketAddr,
}

impl StartKafkaOutletRequest {
    pub fn new(bootstrap_server_addr: SocketAddr) -> Self {
        Self {
            bootstrap_server_addr,
        }
    }

    pub fn bootstrap_server_addr(&self) -> &SocketAddr {
        &self.bootstrap_server_addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaConsumerRequest {
    #[n(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[n(3)] project_route: String,
}

impl StartKafkaConsumerRequest {
    pub fn new(
        bootstrap_server_addr: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        project_route: MultiAddr,
    ) -> Self {
        Self {
            bootstrap_server_addr,
            brokers_port_range: brokers_port_range.into(),
            project_route: project_route.to_string(),
        }
    }

    pub fn bootstrap_server_addr(&self) -> SocketAddr {
        self.bootstrap_server_addr
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> &String {
        &self.project_route
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaProducerRequest {
    #[n(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[n(3)] project_route: String,
}

impl StartKafkaProducerRequest {
    pub fn new(
        bootstrap_server_addr: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        project_route: MultiAddr,
    ) -> Self {
        Self {
            bootstrap_server_addr,
            brokers_port_range: brokers_port_range.into(),
            project_route: project_route.to_string(),
        }
    }

    pub fn bootstrap_server_addr(&self) -> SocketAddr {
        self.bootstrap_server_addr
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> &String {
        &self.project_route
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaDirectRequest {
    #[n(1)] bind_address: SocketAddr,
    #[n(2)] bootstrap_server_addr: SocketAddr,
    #[n(3)] brokers_port_range: (u16, u16),
    #[n(4)] consumer_route: Option<String>,
}

impl StartKafkaDirectRequest {
    pub fn new(
        bind_address: SocketAddr,
        bootstrap_server_addr: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        consumer_route: Option<MultiAddr>,
    ) -> Self {
        Self {
            bind_address,
            bootstrap_server_addr,
            brokers_port_range: brokers_port_range.into(),
            consumer_route: consumer_route.map(|a| a.to_string()),
        }
    }

    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }
    pub fn bootstrap_server_addr(&self) -> &SocketAddr {
        &self.bootstrap_server_addr
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn consumer_route(&self) -> Option<String> {
        self.consumer_route.clone()
    }
}

/// Request body when instructing a node to start an Uppercase service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartUppercaseServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartUppercaseServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body when instructing a node to start an Echoer service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartEchoerServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartEchoerServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body when instructing a node to start a Hop service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartHopServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartHopServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

#[derive(Debug, Clone, Serialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceStatus {
    #[n(2)] pub addr: String,
    #[n(3)] pub service_type: String,
}

impl ServiceStatus {
    pub fn new(addr: impl Into<String>, service_type: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            service_type: service_type.into(),
        }
    }
}

/// Response body for listing services
#[derive(Debug, Clone, Serialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceList {
    #[n(1)] pub list: Vec<ServiceStatus>
}

impl ServiceList {
    pub fn new(list: Vec<ServiceStatus>) -> Self {
        Self { list }
    }
}
