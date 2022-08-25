use std::path::Path;

use minicbor::{bytes::ByteSlice, Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Request body when instructing a node to start a Vault service
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVaultServiceRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9798850>,
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartVaultServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
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
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartIdentityServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
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
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartAuthenticatedServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
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
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartUppercaseServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
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
    #[b(1)] pub addr: Cow<'a, str>,
}

impl<'a> StartEchoerServiceRequest<'a> {
    pub fn new(addr: impl Into<Cow<'a, str>>) -> Self {
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
    #[n(0)] tag: TypeTag<4724285>,
    #[b(1)] addr: &'a str,
    #[b(2)] path: &'a Path,
    #[b(3)] proj: &'a ByteSlice
}

impl<'a> StartAuthenticatorRequest<'a> {
    pub fn new(addr: &'a str, path: &'a Path, proj: &'a [u8]) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
            path,
            proj: proj.into(),
        }
    }

    pub fn address(&self) -> &'a str {
        self.addr
    }

    pub fn path(&self) -> &'a Path {
        self.path
    }

    pub fn project(&self) -> &'a [u8] {
        self.proj
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartVerifierService<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9580740>,
    #[b(1)] addr: &'a str,
}

impl<'a> StartVerifierService<'a> {
    pub fn new(addr: &'a str) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
        }
    }

    pub fn address(&self) -> &'a str {
        self.addr
    }
}

#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct StartCredentialsService<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6467937>,
    #[b(1)] addr: &'a str,
    #[n(2)] oneway: bool,
}

impl<'a> StartCredentialsService<'a> {
    pub fn new(addr: &'a str, oneway: bool) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            addr,
            oneway,
        }
    }

    pub fn address(&self) -> &'a str {
        self.addr
    }

    pub fn oneway(&self) -> bool {
        self.oneway
    }
}
