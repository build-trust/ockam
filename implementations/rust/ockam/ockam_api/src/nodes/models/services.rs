use std::path::Path;

use minicbor::{Decode, Encode};
use ockam_core::{CowBytes, CowStr};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

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
    #[b(2)] path: &'a Path,
    #[b(3)] proj: CowBytes<'a>
}

impl<'a> StartAuthenticatorRequest<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>, path: &'a Path, proj: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr: addr.into(),
            path,
            proj: proj.into(),
        }
    }

    pub fn address(&'a self) -> &'a str {
        &self.addr
    }

    pub fn path(&self) -> &'a Path {
        self.path
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
    #[b(1)] addr: CowStr<'a>,
    #[n(2)] oneway: bool,
}

impl<'a> StartCredentialsService<'a> {
    pub fn new(addr: impl Into<CowStr<'a>>, oneway: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
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

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceStatus<'a> {
    #[cfg(feature = "tag")]
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
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ServiceList<'a> {
    #[cfg(feature = "tag")]
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
