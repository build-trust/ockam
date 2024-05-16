use crate::colors::{color_primary, color_warn};

use crate::output::Output;

use crate::kafka::{ConsumerPublishing, ConsumerResolution};
use minicbor::{Decode, Encode};
use ockam_core::compat::net::SocketAddr;
use ockam_core::Address;
use ockam_multiaddr::MultiAddr;
use serde::Serialize;
use std::fmt::Display;

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
    #[n(1)] bootstrap_server_addr: SocketAddr,
}

impl StartKafkaOutletRequest {
    pub fn new(bootstrap_server_addr: SocketAddr) -> Self {
        Self {
            bootstrap_server_addr,
        }
    }

    pub fn bootstrap_server_addr(&self) -> SocketAddr {
        self.bootstrap_server_addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaInletRequest {
    #[n(1)] bind_address: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[n(3)] kafka_outlet_route: MultiAddr,
    #[n(4)] consumer_resolution: ConsumerResolution,
    #[n(5)] consumer_publishing: ConsumerPublishing,
}

impl StartKafkaInletRequest {
    pub fn new(
        bind_address: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        kafka_outlet_route: MultiAddr,
        consumer_resolution: ConsumerResolution,
        consumer_publishing: ConsumerPublishing,
    ) -> Self {
        Self {
            bind_address,
            brokers_port_range: brokers_port_range.into(),
            kafka_outlet_route,
            consumer_resolution,
            consumer_publishing,
        }
    }

    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> MultiAddr {
        self.kafka_outlet_route.clone()
    }

    pub fn consumer_resolution(&self) -> ConsumerResolution {
        self.consumer_resolution.clone()
    }

    pub fn consumer_publishing(&self) -> ConsumerPublishing {
        self.consumer_publishing.clone()
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
    #[serde(rename = "type")]
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

impl Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} service at {}",
            color_warn(&self.service_type),
            color_primary(&self.addr)
        )
    }
}

impl Output for ServiceStatus {
    fn item(&self) -> crate::Result<String> {
        Ok(format!("{}", self))
    }
}
