use minicbor::{Decode, Encode};
use ockam_core::compat::net::SocketAddr;
use ockam_core::Address;

use serde::Serialize;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartServiceRequest<T> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3470984>,
    #[b(1)] addr: String,
    #[n(2)] req: T,
}

impl<T> StartServiceRequest<T> {
    pub fn new<S: Into<String>>(req: T, addr: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9359178>,
    #[b(1)] addr: String,
}

impl DeleteServiceRequest {
    pub fn new<S: Into<String>>(addr: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }

    pub fn address(&self) -> Address {
        Address::from(self.addr.clone())
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaOutletRequest {
    #[b(1)] pub bootstrap_server_addr: String,
}

impl StartKafkaOutletRequest {
    pub fn new(bootstrap_server_addr: impl Into<String>) -> Self {
        Self {
            bootstrap_server_addr: bootstrap_server_addr.into(),
        }
    }

    pub fn bootstrap_server_addr(&self) -> &str {
        &self.bootstrap_server_addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaConsumerRequest {
    #[b(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[b(3)] project_route: String,
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
    #[b(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[b(3)] project_route: String,
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

/// Request body when instructing a node to start an Identity service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartIdentityServiceRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6129106>,
    #[b(1)] pub addr: String,
}

impl StartIdentityServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Authenticated service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatedServiceRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5179596>,
    #[b(1)] pub addr: String,
}

impl StartAuthenticatedServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Uppercase service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartUppercaseServiceRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8177400>,
    #[b(1)] pub addr: String,
}

impl StartUppercaseServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Echoer service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartEchoerServiceRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7636656>,
    #[b(1)] pub addr: String,
}

impl StartEchoerServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start a Hop service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartHopServiceRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7361428>,
    #[b(1)] pub addr: String,
}

impl StartHopServiceRequest {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartAuthenticatorRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2749734>,
    #[b(1)] addr: String,
    #[b(3)] proj: Vec<u8>,
    // FIXME: test id old format still matches with this
}

impl StartAuthenticatorRequest {
    pub fn new(addr: impl Into<String>, proj: impl Into<Vec<u8>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9580740>,
    #[b(1)] addr: String,
}

impl StartVerifierService {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }

    pub fn address(&self) -> &str {
        &self.addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartCredentialsService {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6467937>,
    #[b(1)] public_identity: String,
    #[b(2)] addr: String,
    #[n(3)] oneway: bool,
}

impl StartCredentialsService {
    pub fn new(public_identity: impl Into<String>, addr: impl Into<String>, oneway: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2291842>,
    #[b(1)] addr: String,
    #[b(2)] tenant_base_url: String,
    #[b(3)] certificate: String,
    #[b(4)] attributes: Vec<String>,
    #[b(5)] proj: Vec<u8>
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
            #[cfg(feature = "tag")]
            tag: TypeTag,
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
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] tag: TypeTag<8542064>,
    #[b(2)] pub addr: String,
    #[b(3)] pub service_type: String,
}

impl ServiceStatus {
    pub fn new(addr: impl Into<String>, service_type: impl Into<String>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] tag: TypeTag<9587601>,
    #[b(1)] pub list: Vec<ServiceStatus>
}

impl ServiceList {
    pub fn new(list: Vec<ServiceStatus>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
