use minicbor::{Decode, Encode};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{CowBytes, CowStr};

use serde::Serialize;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_multiaddr::MultiAddr;

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartServiceRequest<'a, T> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3470984>,
    #[b(1)] addr: CowStr<'a>,
    #[n(2)] req: T,
}

impl<'a, T> StartServiceRequest<'a, T> {
    pub fn new<S: Into<CowStr<'a>>>(req: T, addr: S) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
            req,
        }
    }

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }

    pub fn request(&'a self) -> &'a T {
        &self.req
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaConsumerRequest<'a> {
    #[b(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[b(3)] project_route: CowStr<'a>,
}

impl<'a> StartKafkaConsumerRequest<'a> {
    pub fn new(
        bootstrap_server_addr: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        project_route: MultiAddr,
    ) -> Self {
        Self {
            bootstrap_server_addr,
            brokers_port_range: brokers_port_range.into(),
            project_route: project_route.to_string().into(),
        }
    }

    pub fn bootstrap_server_addr(&self) -> SocketAddr {
        self.bootstrap_server_addr
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> &CowStr<'a> {
        &self.project_route
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartKafkaProducerRequest<'a> {
    #[b(1)] pub bootstrap_server_addr: SocketAddr,
    #[n(2)] brokers_port_range: (u16, u16),
    #[b(3)] project_route: CowStr<'a>,
}

impl<'a> StartKafkaProducerRequest<'a> {
    pub fn new(
        bootstrap_server_addr: SocketAddr,
        brokers_port_range: impl Into<(u16, u16)>,
        project_route: MultiAddr,
    ) -> Self {
        Self {
            bootstrap_server_addr,
            brokers_port_range: brokers_port_range.into(),
            project_route: project_route.to_string().into(),
        }
    }

    pub fn bootstrap_server_addr(&self) -> SocketAddr {
        self.bootstrap_server_addr
    }
    pub fn brokers_port_range(&self) -> (u16, u16) {
        self.brokers_port_range
    }
    pub fn project_route(&self) -> &CowStr<'a> {
        &self.project_route
    }
}

/// Request body when instructing a node to start a Vault service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVaultServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9798850>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartVaultServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }
}

/// Request body when instructing a node to start an Identity service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartIdentityServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6129106>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartIdentityServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct StartAuthenticatedServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5179596>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartAuthenticatedServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct StartUppercaseServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8177400>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartUppercaseServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct StartEchoerServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7636656>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartEchoerServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct StartHopServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7361428>,
    #[b(1)] pub addr: CowStr<'a>,
}

impl<'a> StartHopServiceRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
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
pub struct StartAuthenticatorRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2749734>,
    #[b(1)] addr: CowStr<'a>,
    #[b(3)] proj: CowBytes<'a>,
    // FIXME: test id old format still matches with this
}

impl<'a> StartAuthenticatorRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>, proj: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
            proj: proj.into(),
        }
    }

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }

    pub fn project(&'a self) -> &'a [u8] {
        &self.proj
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVerifierService<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9580740>,
    #[b(1)] addr: CowStr<'a>,
}

impl<'a> StartVerifierService<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
        }
    }

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartCredentialsService<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6467937>,
    #[b(1)] public_identity: CowStr<'a>,
    #[b(2)] addr: CowStr<'a>,
    #[n(3)] oneway: bool,
}

impl<'a> StartCredentialsService<'a> {
    pub fn new(
        public_identity: impl Into<CowStr<'a>>,
        addr: impl Into<CowStr<'a>>,
        oneway: bool,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            public_identity: public_identity.into(),
            addr: addr.into(),
            oneway,
        }
    }

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }

    pub fn oneway(&self) -> bool {
        self.oneway
    }

    pub fn public_identity(&'a self) -> &'a str {
        &self.public_identity
    }
}

/// Request body when instructing a node to start an Okta Identity Provider service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartOktaIdentityProviderRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2291842>,
    #[b(1)] addr: CowStr<'a>,
    #[b(2)] tenant_base_url: CowStr<'a>,
    #[b(3)] certificate: CowStr<'a>,
    #[b(4)] attributes: Vec<&'a str>,
    #[b(5)] proj: CowBytes<'a>
}

impl<'a> StartOktaIdentityProviderRequest<'a> {
    pub fn new(
        addr: impl Into<CowStr<'a>>,
        tenant_base_url: impl Into<CowStr<'a>>,
        certificate: impl Into<CowStr<'a>>,
        attributes: Vec<&'a str>,
        proj: impl Into<CowBytes<'a>>,
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

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }
    pub fn tenant_base_url(&'a self) -> &'a str {
        &self.tenant_base_url
    }
    pub fn certificate(&'a self) -> &'a str {
        &self.certificate
    }
    pub fn project(&'a self) -> &'a [u8] {
        &self.proj
    }
    pub fn attributes(&self) -> &[&str] {
        &self.attributes
    }
}

#[derive(Debug, Clone, Serialize, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceStatus<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] tag: TypeTag<8542064>,
    #[b(2)] pub addr: CowStr<'a>,
    #[b(3)] pub service_type: CowStr<'a>,
}

impl<'a> ServiceStatus<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>, service_type: impl Into<CowStr<'a>>) -> Self {
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
pub struct ServiceList<'a> {
    #[cfg(feature = "tag")]
    #[serde(skip_serializing)]
    #[n(0)] tag: TypeTag<9587601>,
    #[b(1)] pub list: Vec<ServiceStatus<'a>>
}

impl<'a> ServiceList<'a> {
    pub fn new(list: Vec<ServiceStatus<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            list,
        }
    }
}
