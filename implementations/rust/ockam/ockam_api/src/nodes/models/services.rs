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

/// Request body when instructing a node to start an Identity service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartIdentityServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartIdentityServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }
}

/// Request body when instructing a node to start an Authenticated service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatedServiceRequest {
    #[n(1)] pub addr: String,
}

impl StartAuthenticatedServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
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

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatorRequest {
    #[n(1)] addr: String,
    #[n(3)] proj: Vec<u8>,
    // FIXME: test id old format still matches with this
}

impl StartAuthenticatorRequest {
    pub fn new(addr: impl Into<String>, proj: impl Into<Vec<u8>>) -> Self {
        Self {
            addr: addr.into(),
            proj: proj.into(),
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }

    pub fn project(&self) -> &[u8] {
        &self.proj
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVerifierService {
    #[n(1)] addr: String,
}

impl StartVerifierService {
    pub fn new(addr: impl Into<String>) -> Self {
        Self { addr: addr.into() }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartCredentialsService {
    #[n(1)] public_identity: String,
    #[n(2)] addr: String,
    #[n(3)] oneway: bool,
}

impl StartCredentialsService {
    pub fn new(public_identity: impl Into<String>, addr: impl Into<String>, oneway: bool) -> Self {
        Self {
            public_identity: public_identity.into(),
            addr: addr.into(),
            oneway,
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }

    pub fn oneway(&self) -> bool {
        self.oneway
    }

    pub fn public_identity(&self) -> &str {
        &self.public_identity
    }
}

/// Request body when instructing a node to start an Okta Identity Provider service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartOktaIdentityProviderRequest {
    #[n(1)] addr: String,
    #[n(2)] tenant_base_url: String,
    #[n(3)] certificate: String,
    #[n(4)] attributes: Vec<String>,
    #[n(5)] proj: Vec<u8>
}

impl StartOktaIdentityProviderRequest {
    pub fn new(
        addr: impl Into<String>,
        tenant_base_url: impl Into<String>,
        certificate: impl Into<String>,
        attributes: Vec<String>,
        proj: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            addr: addr.into(),
            tenant_base_url: tenant_base_url.into(),
            certificate: certificate.into(),
            attributes,
            proj: proj.into(),
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }
    pub fn tenant_base_url(&self) -> &str {
        &self.tenant_base_url
    }
    pub fn certificate(&self) -> &str {
        &self.certificate
    }
    pub fn project(&self) -> &[u8] {
        &self.proj
    }
    pub fn attributes(&self) -> &[String] {
        &self.attributes
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
